use std::collections::HashMap;
use std::io::{BufRead, BufReader, Result};

type Clause = Vec<Literal<usize>>;

pub struct EncoderSAT<T> {
    map: HashMap<T, usize>,
    clauses: Vec<Clause>,
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

impl<T> EncoderSAT<T> {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

impl<T: Eq + std::hash::Hash> EncoderSAT<T> {
    pub fn add(&mut self, clause: Vec<Literal<T>>) {
        self.clauses.push(
            clause
                .into_iter()
                .map(|literal| {
                    let next_id = self.map.len() + 1;
                    literal.map(|t| *self.map.entry(t).or_insert(next_id))
                })
                .collect(),
        );
    }
}

impl<T> Default for EncoderSAT<T> {
    fn default() -> Self {
        Self {
            map: Default::default(),
            clauses: Default::default(),
        }
    }
}

// TODO: return a printable string
impl<T: std::fmt::Debug + Clone> EncoderSAT<T> {
    pub fn encode(&self) -> (String, Vec<T>) {
        let variables_number = self.map.len();

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

    pub fn clause(self) -> ClauseBuilder<T> {
        ClauseBuilder {
            encoder: self,
            clause: Default::default(),
        }
    }
}

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
    T: std::cmp::Eq + std::hash::Hash,
{
    pub fn add<U: Into<Literal<T>>>(&mut self, literal: U) {
        let next_id = self.encoder.map.len() + 1;
        let literal = literal
            .into()
            .map(|t| *self.encoder.map.entry(t).or_insert(next_id));

        self.clause.push(literal);
    }

    pub fn end(mut self) -> EncoderSAT<T> {
        self.encoder.clauses.push(self.clause);
        self.encoder
    }
}
