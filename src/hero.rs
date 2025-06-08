use std::{collections::HashSet, fmt, process::exit};

use rand::{Rng, rngs::ThreadRng};

use crate::{
    encoder::Literal,
    kb::{Formula, KnowledgeBase, Var},
    world::{Action, Perceptions, Position},
};

enum Objective {
    TakeGold,
    GoHome,
}

pub struct Hero<K> {
    kb: K,
    obj: Objective,
    t: usize, // time
    visited: HashSet<Position>,
    dangeours: HashSet<Position>,
    safe: HashSet<Position>,
    rng: ThreadRng,
}

impl<K> Hero<K> {
    pub fn new(kb: K) -> Self {
        let mut safe = HashSet::new();
        safe.insert(Position::new(0, 0));
        Self {
            kb: kb,
            t: 0,
            visited: HashSet::new(),
            dangeours: HashSet::new(),
            safe: safe,
            rng: rand::rng(),
            obj: Objective::TakeGold,
        }
    }

    fn utility_take_gold(&mut self, a: &Action, p: &Position) -> i32 {
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

    fn utility_go_home(&mut self, a: &Action, p: &Position) -> i32 {
        match *a {
            Action::Move(direction) => {
                // costruisci un piano da dove ti trovi adesso fino alla destinazione (0,0)
                // preferisci le celle che ti avicinano in quel piano
                todo!()
            }
            Action::Grab => i32::MAX,
            Action::Shoot(direction) => todo!(),
        }
    }

    fn utility(&mut self, a: &Action, p: &Position) -> i32 {
        match self.obj {
            Objective::TakeGold => self.utility_take_gold(a, p),
            Objective::GoHome => self.utility_go_home(a, p),
        }
    }
}

impl<K: KnowledgeBase<Query: fmt::Debug>> Hero<K> {
    pub fn next_action(&mut self, p: Perceptions) -> Action {
        use crate::world::Action::*;
        use crate::world::Direction::*;

        println!("{:?}", p);

        if !self.kb.consistency() {
            println!("[FATAL ERROR] Inconsistency found in the knowledge base");
            exit(1);
        }

        self.kb.tell(&K::create_ground_truth_from_perception(&p));
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
            self.obj = Objective::GoHome;
        }

        // TODO: add arrow

        for a in action_to_consider {
            let formula = K::create_query_from_action(&a, &p.position);
            if self.kb.ask(&formula) {
                println!("Inferito: {:?}", formula);
                suitable_actions.push(a);
                self.kb.tell(&formula);
                for pos in self.kb.safe_positions(formula).into_iter() {
                    self.safe.insert(pos);
                }
            } else {
                match a {
                    Move(dir) => {
                        if self.kb.is_unsafe(p.position.move_clone(dir)) {
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
