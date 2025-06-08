use std::collections::HashSet;

use rand::{Rng, rngs::ThreadRng};

use crate::{
    encoder::Literal,
    kb::{Formula, KnowledgeBase, Var},
    world::{Action, Perceptions, Position},
};

pub struct Hero {
    kb: Box<dyn KnowledgeBase>,
    t: usize, // time
    visited: HashSet<Position>,
    dangeours: HashSet<Position>,
    safe: HashSet<Position>,
    rng: ThreadRng,
}

impl Hero {
    pub fn new(kb: Box<dyn KnowledgeBase>) -> Self {
        Self {
            kb: kb,
            t: 0,
            visited: HashSet::new(),
            dangeours: HashSet::new(),
            safe: HashSet::new(),
            rng: rand::rng(),
        }
    }

    fn create_formula_perception(&self, p: &Perceptions) -> Formula {
        use crate::kb::Var::*;

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

    fn create_formula_ask(&self, a: &Action, pos: &Position) -> Formula {
        use crate::kb::Var::*;

        match *a {
            Action::Move(direction) => vec![vec![
                Safe {
                    pos: pos.move_clone(direction),
                }
                .into(),
            ]],
            Action::Grab => vec![vec![Gold { pos: *pos }.into()]],
            Action::Shoot(direction) => todo!(),
        }
    }

    fn create_action_tell(&self, a: &Action) -> Formula {
        todo!()
    }

    fn utility(&self, a: &Action, p: &Position) -> i32 {
        match *a {
            Action::Move(direction) => {
                if self.visited.contains(&p.move_clone(direction)) {
                    -1
                } else {
                    0
                }
            }
            Action::Grab => i32::MAX,
            Action::Shoot(direction) => todo!(),
        }
    }

    pub fn next_action(&mut self, p: Perceptions) -> Action {
        use crate::world::Action::*;
        use crate::world::Direction::*;

        println!("{:?}", p);

        self.kb.tell(&self.create_formula_perception(&p));
        let mut suitable_actions = vec![];
        let mut action_to_consider = Vec::with_capacity(9);

        for dir in [North, Sud, East, Ovest] {
            if p.position.possible_move(dir, p.board_size)
                && !self.dangeours.contains(&p.position.move_clone(dir))
            {
                if self.safe.contains(&p.position.move_clone(dir)) {
                    suitable_actions.push(Move(dir));
                } else {
                    action_to_consider.push(Move(dir));
                }
            }
        }

        if p.glitter {
            action_to_consider.push(Grab);
        }

        // TODO: add arrow

        for a in action_to_consider {
            let formula = self.create_formula_ask(&a, &p.position);
            if self.kb.ask(&formula) {
                println!("Inferito: {:?}", formula);
                suitable_actions.push(a);
                self.kb.tell(&formula);
                for clause in formula {
                    for literal in clause.into_iter().map(|x| x.inner()) {
                        match literal {
                            Var::Safe { pos } => {
                                self.safe.insert(pos);
                            }
                            _ => {}
                        }
                    }
                }
            } else {
                match a {
                    Move(dir) => {
                        if self.kb.ask(&vec![vec![
                            Var::Wumpus {
                                pos: p.position.move_clone(dir),
                            }
                            .into(),
                        ]]) {
                            self.kb.tell(&vec![vec![
                                Var::Wumpus {
                                    pos: p.position.move_clone(dir),
                                }
                                .into(),
                            ]]);
                            println!(
                                "Ci sta il wumpus in posizione: {:?}",
                                p.position.move_clone(dir)
                            );
                            self.dangeours.insert(p.position.move_clone(dir));
                        } else if self.kb.ask(&vec![vec![
                            Var::Pit {
                                pos: p.position.move_clone(dir),
                            }
                            .into(),
                        ]]) {
                            self.kb.tell(&vec![vec![
                                Var::Pit {
                                    pos: p.position.move_clone(dir),
                                }
                                .into(),
                            ]]);
                            println!(
                                "Ci sta un pozzo in posizione: {:?}",
                                p.position.move_clone(dir)
                            );
                            self.dangeours.insert(p.position.move_clone(dir));
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut best = None;
        let mut best_utility = i32::MIN;
        for action in suitable_actions {
            let new_utility = self.utility(&action, &p.position);
            if new_utility > best_utility {
                best = action.into();
                best_utility = new_utility;
            } else if new_utility == best_utility {
                if self.rng.random_bool(0.5) {
                    best = action.into();
                }
            }
        }
        if let Some(a) = best {
            // self.kb.tell(self.create_action_tell(&a));
            self.t += 1;
            self.visited.insert(p.position);
            return a;
        } else {
            panic!("no action possible");
        }
    }
}
