use crate::{
    encoder::{EncoderSAT, Literal, Literal::Neg},
    world::{Direction, Position},
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
    // @return true iff KB |= formula
    fn ask(&mut self, formula: &Formula) -> bool;
    fn tell(&mut self, formula: &Formula);
    fn consistency(&mut self) -> bool;
}

impl KnowledgeBase for EncoderSAT<Var> {
    fn ask(&mut self, formula: &Formula) -> bool {
        let result: bool;
        self.snapshot(); // prendi una foto dello stato della KB
        if formula.len() > 1 {
            let mut tseytin_clause = vec![];
            for clause in formula {
                // la formula da aggiungere alla KB è (t_1 or t_2 or ... or t_n) and (t_1 <-> not c_1) and ... and (t_n <-> not c_2)
                // che sarebbe inferenzialmente equivalente alla formula di input negata
                // crea una variabile di tseitin t per clausola
                // aggiungi alla KB la clausola (t or clausola)
                // siano alpha_1 or alpha_2 or ... or alpha_k i letterali della clausola
                // aggiungi alla KB le clausole (not t or not alpha_1) and ... and (not t or not alpha_k)
                // aggiungi la clausola (t_1 or t_2 or ... or t_n) dove n è il numero di clausole.
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
