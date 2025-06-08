use crate::{
    encoder::{
        EncoderSAT,
        Literal::{self, Neg},
    },
    world::{Action, Direction, Perceptions, Position},
};

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub enum Var {
    Safe { pos: Position },
    Wumpus { pos: Position },
    Pit { pos: Position },
    Gold { pos: Position },
    Stench { pos: Position },
    Breeze { pos: Position },
    Howl,
    Bump { pos: Position, dir: Direction },
}

impl Default for Var {
    fn default() -> Self {
        Self::Safe {
            pos: Position::default(),
        }
    }
}

pub type Formula = Vec<Vec<Literal<Var>>>;

pub trait KnowledgeBase {
    type Query;

    // @return true iff KB |= formula
    fn ask(&mut self, formula: &Self::Query) -> bool;
    fn tell(&mut self, formula: &Self::Query);

    fn consistency(&mut self) -> bool;

    fn create_query_from_action(a: &Action, p: &Position) -> Self::Query;
    fn create_ground_truth_from_perception(p: &Perceptions) -> Self::Query;

    fn is_unsafe(&mut self, p: Position) -> bool;
    fn safe_positions(&self, query: Self::Query) -> Vec<Position>;
}

impl KnowledgeBase for EncoderSAT<Var> {
    type Query = Formula;

    fn ask(&mut self, formula: &Formula) -> bool {
        let result: bool;
        self.snapshot(); // prendi una foto dello stato della KB
        if formula.len() > 1 {
            let mut tseytin_clause = vec![];
            for clause in formula {
                // la formula da aggiungere alla KB è (t_1 or t_2 or ... or t_n) and (t_1 <-> not c_1) and ... and (t_n <-> not c_2)
                // dove c_1, c_2, ..., c_n sono le clausole della formula in input originale (non negata)
                // Questa nuova formula è inferenzialmente equivalente alla formula di partenza negata.
                // Però dobbiamo renderla in CNF in questo modo:
                // Crea una variabile di tseytin t_i per clausola c_i
                // aggiungi alla KB la clausola (t_i or c_i)
                // siano alpha_1 or alpha_2 or ... or alpha_k i letterali della clausola c_i
                // aggiungi alla KB le clausole (not t_i or not alpha_1) and ... and (not t_i or not alpha_k)
                // aggiungi la clausola (t_1 or t_2 or ... or t_n)
                let tseytin = self.create_raw_variable();
                tseytin_clause.push(tseytin.clone());
                for literal in clause {
                    let not_literal = self.register_literal(literal.not());
                    let not_tseytin = tseytin.not();
                    self.add_raw_clause(vec![not_literal, not_tseytin]);
                }
                let mut raw_clause = self.register_clause(clause.clone());
                raw_clause.push(tseytin.clone());
                self.add_raw_clause(raw_clause); // aggiunta clausola t or clausola
            }
            self.add_raw_clause(tseytin_clause);
        } else {
            if let Some(clause) = formula.get(0) {
                for literal in clause {
                    self.add(vec![literal.not()]);
                }
            } else {
                self.rewind(); // rimuovi lo snapshot
                return false;
            }
        }
        result = !self.picosat_sat(); // TODO: generalize for all the solvers
        self.rewind(); // rimuovi le modifiche e lo snapshot della KB
        return result;
    }

    fn tell(&mut self, formula: &Formula) {
        for clause in formula {
            self.add(clause.clone());
        }
    }

    fn consistency(&mut self) -> bool {
        let result = self.picosat_sat();
        if !result {
            println!("{:?}", self);
        }
        result
    }

    fn create_query_from_action(a: &Action, p: &Position) -> Self::Query {
        use Var::*;

        match *a {
            Action::Move(direction) => vec![vec![
                Safe {
                    pos: p.move_clone(direction),
                }
                .into(),
            ]],
            Action::Grab => vec![vec![Gold { pos: *p }.into()]],
            Action::Shoot(direction) => todo!(),
        }
    }
    fn create_ground_truth_from_perception(p: &Perceptions) -> Self::Query {
        use Var::*;

        let mut formula = Vec::new();
        let mut var: Literal<Var> = Breeze { pos: p.position }.into();
        if !p.breeze {
            var = var.not();
        }
        formula.push(vec![var]);
        var = Gold { pos: p.position }.into();
        if p.glitter {
            formula.push(vec![var]);
        }
        var = Stench { pos: p.position }.into();
        if !p.stench {
            var = var.not();
        }
        formula.push(vec![var]);

        // TODO: bump and howl

        formula
    }

    fn is_unsafe(&mut self, p: Position) -> bool {
        use Var::*;

        let phi = vec![vec![Wumpus { pos: p }.into(), Pit { pos: p }.into()]];

        if self.ask(&phi) {
            self.tell(&phi);
            println!("[INFO] Position {:?} is UNSAFE", p);
            if self.ask(&vec![vec![Pit { pos: p }.into()]]) {
                self.tell(&vec![vec![Pit { pos: p }.into()]]);
                println!("[INFO] Pit in position: {:?}", p);
            } else {
                self.tell(&vec![vec![Wumpus { pos: p }.into()]]);
                println!("[INFO] Wumpus in position: {:?}", p);
            };

            return true;
        }

        return false;
    }

    fn safe_positions(&self, query: Self::Query) -> Vec<Position> {
        let mut result = vec![];
        for clause in query {
            for literal in clause.into_iter().map(|x| x.inner()) {
                match literal {
                    Var::Safe { pos } => {
                        result.push(pos);
                    }
                    _ => {}
                }
            }
        }
        result
    }
}

pub fn init_kb(size: usize) -> EncoderSAT<Var> {
    use Var::*;

    let mut kb = EncoderSAT::new();

    // il wumpus esiste in almeno una posizione

    let mut clause = kb.clause();

    for i in 0..size {
        for j in 0..size {
            clause.add(Wumpus {
                pos: Position { x: i, y: j },
            });
            // println!("i,j: {:?}", (i, j));
        }
    }
    kb = clause.end();
    println!("[INFO] At least one Wumpus");

    // la stanza 0 0 è sicura
    clause = kb.clause();
    clause.add(Safe {
        pos: Position::new(0, 0),
    });
    kb = clause.end();
    println!("[INFO] The cell 0 0 is safe");

    // il wumpus si trova in esattamente una posizione
    // il wumpus non si può trovare in due posizioni diverse

    for i in 0..size {
        for j in 0..size {
            for x in 0..size {
                for y in 0..size {
                    if (i, j) != (x, y) {
                        let pos1 = Position::new(i, j);
                        let pos2 = Position::new(x, y);
                        // il wumpus si trova in esattamente una posizione
                        // il wumpus non si può trovare in due posizioni diverse
                        clause = kb.clause();
                        clause.add(Neg(Wumpus { pos: pos1 }));
                        clause.add(Neg(Wumpus { pos: pos2 }));
                        kb = clause.end();
                        // l'oro si trova esattamente in una posizone
                        // l'oro non si può trovare in due posizioni diverse
                        clause = kb.clause();
                        clause.add(Neg(Gold { pos: pos1 }));
                        clause.add(Neg(Gold { pos: pos2 }));
                        kb = clause.end();
                    }
                }
            }
        }
    }

    println!("[INFO] at most one wumpus and one gold");

    // l'oro si trova in almeno una posizione
    clause = kb.clause();
    for i in 0..size {
        for j in 0..size {
            clause.add(Gold {
                pos: Position { x: i, y: j },
            });
        }
    }
    kb = clause.end();
    println!("[INFO] at least one gold");

    // in una stanza c'è vento se e solo se in una stanza adiacente c'è il pozzo
    let mut vento_implica_pozzi = vec![];
    // let mut pozzo_implica_vento = vec![];
    // in una stanza c'è puzza se e solo se in una stanza adiacente c'è il Wumpus
    let mut puzza_implica_wumpus = vec![];
    // let mut wumpus_implica_puzza = vec![];

    use crate::world::Direction::*;

    for i in 0..size {
        for j in 0..size {
            let pos = Position::new(i, j);
            vento_implica_pozzi.push(Neg(Breeze { pos: pos }));
            puzza_implica_wumpus.push(Neg(Stench { pos: pos }));
            for dir in [North, Sud, East, Ovest] {
                if pos.possible_move(dir, size) {
                    // vento_implica_pozzo
                    clause = kb.clause();
                    clause.add(Neg(Pit { pos: pos }));
                    clause.add(Breeze {
                        pos: pos.move_clone(dir),
                    });
                    kb = clause.end();
                    vento_implica_pozzi.push(
                        Pit {
                            pos: pos.move_clone(dir),
                        }
                        .into(),
                    );
                    // puzza_implica_wumpus
                    clause = kb.clause();
                    clause.add(Neg(Wumpus { pos: pos }));
                    clause.add(Stench {
                        pos: pos.move_clone(dir),
                    });
                    kb = clause.end();
                    puzza_implica_wumpus.push(
                        Wumpus {
                            pos: pos.move_clone(dir),
                        }
                        .into(),
                    );
                }
            }
            kb.add(vento_implica_pozzi);
            kb.add(puzza_implica_wumpus);
            vento_implica_pozzi = vec![];
            puzza_implica_wumpus = vec![];
        }
    }

    println!("[INFO] physics of the world");

    // se una casella è safe allora non c'è il wumpus e non c'è il pozzo
    // se in una casella non c'è il wumpus e non c'è il pozzo allora è safe
    // se in una casella non c'è un pozzo allora è safe
    for i in 0..size {
        for j in 0..size {
            clause = kb.clause();
            clause.add(Safe {
                pos: Position::new(i, j),
            });
            clause.add(Wumpus {
                pos: Position::new(i, j),
            });
            clause.add(Pit {
                pos: Position::new(i, j),
            });
            kb = clause.end();
            clause = kb.clause();
            clause.add(Neg(Safe {
                pos: Position::new(i, j),
            }));
            clause.add(Neg(Pit {
                pos: Position::new(i, j),
            }));
            kb = clause.end();
            clause = kb.clause();
            clause.add(Neg(Safe {
                pos: Position::new(i, j),
            }));
            clause.add(Neg(Wumpus {
                pos: Position::new(i, j),
            }));
            kb = clause.end();
        }
    }

    println!("[INFO] safety rules");

    // se il wumpus ha urlato, allora la cella dove stava il wumpus è sicura
    // println!("{:?}", kb);
    // se ha sentito il rumore della freccia sbattere, allora in tutte le celle in cui è passata la freccia non ci sta il wumpus
    kb
}
