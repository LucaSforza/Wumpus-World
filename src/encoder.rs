use std::collections::HashMap;
use std::fmt;
use std::io::{BufRead, BufReader, Result, Write};
use std::process::{Command, Stdio};

type Clause = Vec<Literal<usize>>;

#[derive(Debug)]
struct Snapshot<T> {
    last_var_counter: usize,
    last_len_clauses: usize,
    new_vars: Vec<T>,
}

impl<T> From<&mut EncoderSAT<T>> for Snapshot<T> {
    fn from(value: &mut EncoderSAT<T>) -> Self {
        Self {
            last_var_counter: value.counter,
            last_len_clauses: value.clauses.len(),
            new_vars: Vec::new(),
        }
    }
}

#[derive(Default)]
pub struct EncoderSAT<T> {
    map: HashMap<T, usize>,
    clauses: Vec<Clause>,
    counter: usize,
    snapshot: Option<Snapshot<T>>,
}

impl<T: Clone + Eq + std::hash::Hash + fmt::Debug> fmt::Debug for EncoderSAT<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Build reverse map: usize -> T
        let mut reverse_map: HashMap<usize, &T> = HashMap::new();
        for (t, &id) in &self.map {
            reverse_map.insert(id, t);
        }

        for (i, clause) in self.clauses.iter().enumerate() {
            write!(f, "Clause {}: ", i + 1)?;
            for literal in clause {
                match literal {
                    Literal::Pos(id) => {
                        if let Some(t) = reverse_map.get(id) {
                            write!(f, "{:?} ", t)?;
                        } else {
                            write!(f, "+?({}) ", id)?;
                        }
                    }
                    Literal::Neg(id) => {
                        if let Some(t) = reverse_map.get(id) {
                            write!(f, "-{:?} ", t)?;
                        } else {
                            write!(f, "-?({}) ", id)?;
                        }
                    }
                }
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

pub fn picosat_is_sat(output: String) -> bool {
    let mut reader = BufReader::new(output.as_bytes());

    let mut line = String::new();
    // Read first line
    if reader
        .read_line(&mut line)
        .expect("Could not read the output")
        == 0
    {
        panic!("Could not read first line of the output file");
    }
    if line.trim() != "s SATISFIABLE" {
        false
    } else {
        true
    }
}

/// Parses the PicoSAT output file and returns a Vec<Option<bool>> where
/// index 0 is unused, and each index i corresponds to variable i.
pub fn parse_picosat_model(output: String, nvars: usize) -> Result<Vec<Option<bool>>> {
    let mut reader = BufReader::new(output.as_bytes());

    let mut line = String::new();
    // Read first line
    if reader.read_line(&mut line)? == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Could not read first line of the output file",
        ));
    }
    if line.trim() != "s SATISFIABLE" {
        return Ok(vec![]); // UNSAT
    }

    // Prepare result vector: index 0 is unused
    let mut result = vec![None; nvars + 1];

    for line in reader.lines() {
        let line = line?;
        if !line.starts_with("v ") {
            continue;
        }
        for lit in line[2..].split_whitespace() {
            if let Ok(lit) = lit.parse::<i32>() {
                if lit == 0 {
                    continue;
                }
                let idx = lit.abs() as usize;
                if idx <= nvars {
                    result[idx] = Some(lit > 0);
                }
            }
        }
    }
    Ok(result)
}

/// Given the model (as returned by parse_picosat_model) and the variable dictionary,
/// returns a Vec of (T, Option<bool>) for each variable (excluding index 0).
pub fn decode_model<T: Clone>(vars: &[T], model: &[Option<bool>]) -> Vec<(T, Option<bool>)> {
    vars.iter()
        .cloned()
        .enumerate()
        .skip(1)
        .map(|(i, v)| (v, model.get(i).cloned().unwrap_or(None)))
        .collect()
}

impl<T: fmt::Debug> EncoderSAT<T> {
    pub fn create_raw_variable(&mut self) -> Literal<usize> {
        self.counter += 1;
        self.counter.into()
    }

    pub fn add_raw_clause(&mut self, raw_clause: Clause) {
        self.clauses.push(raw_clause);
    }

    pub fn snapshot(&mut self) {
        assert!(
            self.snapshot.is_none(),
            "there is a snapshot in the Encoder, please consider rewinding before taking another snaposhot"
        );
        self.snapshot = Snapshot::from(&mut *self).into();
        // println!("{:?}", self.snapshot);
    }
}

impl<T: Default> EncoderSAT<T> {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

impl<T: Eq + std::hash::Hash + Clone + fmt::Debug> EncoderSAT<T> {
    pub fn add(&mut self, clause: Vec<Literal<T>>) {
        let clause = self.register_clause(clause);
        self.clauses.push(clause);
    }

    pub fn register_literal(&mut self, literal: Literal<T>) -> Literal<usize> {
        let old_size = self.map.len();
        let next_id = self.counter + 1;
        let result = literal
            .clone() // ugly :(
            .map(|t| *self.map.entry(t).or_insert(next_id));
        if self.map.len() > old_size {
            self.counter += 1;
            if let Some(snapshot) = self.snapshot.as_mut() {
                snapshot.new_vars.push(literal.inner());
            }
        }
        result
    }

    pub fn register_clause(&mut self, clause: Vec<Literal<T>>) -> Clause {
        clause
            .into_iter()
            .map(|literal| self.register_literal(literal))
            .collect()
    }

    pub fn rewind(&mut self) {
        let snapshot = self
            .snapshot
            .as_ref()
            .expect("rewinding the Endored without a snapshot");
        // println!("Rewind: {:?}, new len: {}", snapshot, self.clauses.len());
        self.counter = snapshot.last_var_counter;
        while snapshot.last_len_clauses < self.clauses.len() {
            // TODO: controllare se esiste un modo O(1) per fare la stessa cosa
            self.clauses.pop();
        }
        for var in &snapshot.new_vars {
            self.map.remove(var);
        }
        self.snapshot = None;
    }
}

impl<T: Clone> EncoderSAT<T> {
    pub fn encode(&self) -> (String, Vec<T>) {
        let variables_number = self.counter;

        let mut variables = vec![None; variables_number];
        for (k, v) in &self.map {
            variables[v - 1] = Some(k.clone());
        }

        let variables = variables.into_iter().filter_map(|x| x).collect();

        let mut encoding = String::new();

        encoding.push_str(&format!(
            "p cnf {variables_number} {}\n",
            self.clauses.len()
        ));

        for clause in &self.clauses {
            let mut clause: String = clause
                .into_iter()
                .map(|literal| match literal {
                    Literal::Pos(l) => format!("{l} "),
                    Literal::Neg(l) => format!("-{l} "),
                })
                .collect();
            clause.push('0');
            encoding.push_str(&format!("{clause}\n"));
        }

        (encoding, variables)
    }

    pub fn picosat_sat(&self) -> bool {
        let (encoding, _) = self.encode();
        let output = Command::new("picosat")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                {
                    let stdin = child.stdin.as_mut().expect("Failed to open stdin");
                    stdin.write_all(encoding.as_bytes())?;
                }
                let output = child.wait_with_output()?;
                Ok(output)
            })
            .expect("Failed to run picosat");

        let picosat_stdout = String::from_utf8_lossy(&output.stdout).to_string();

        picosat_is_sat(picosat_stdout)
    }

    pub fn clause(self) -> ClauseBuilder<T> {
        ClauseBuilder {
            encoder: self,
            clause: Default::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Literal<T> {
    Pos(T),
    Neg(T),
}

impl<T> Literal<T> {
    fn map<U, F>(self, f: F) -> Literal<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Literal::Pos(t) => Literal::Pos(f(t)),
            Literal::Neg(t) => Literal::Neg(f(t)),
        }
    }

    pub fn inner(self) -> T {
        match self {
            Literal::Pos(t) => t,
            Literal::Neg(t) => t,
        }
    }
}

impl<T: Copy> Literal<T> {
    pub fn not(&self) -> Self {
        match self {
            Literal::Pos(t) => Literal::Neg(*t),
            Literal::Neg(t) => Literal::Pos(*t),
        }
    }
}

impl<T> From<T> for Literal<T> {
    fn from(value: T) -> Self {
        Literal::Pos(value)
    }
}

pub struct ClauseBuilder<T> {
    encoder: EncoderSAT<T>,
    clause: Clause,
}

impl<T> ClauseBuilder<T>
where
    T: std::cmp::Eq + std::hash::Hash + Clone + fmt::Debug,
{
    pub fn add<U: Into<Literal<T>>>(&mut self, literal: U) {
        self.clause
            .push(self.encoder.register_literal(literal.into()));
    }

    pub fn end(mut self) -> EncoderSAT<T> {
        self.encoder.clauses.push(self.clause);
        self.encoder
    }
}
