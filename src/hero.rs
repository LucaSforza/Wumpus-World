use std::{collections::HashSet, fmt, process::exit, ptr::eq};

use bumpalo::Bump;
use rand::{Rng, rngs::ThreadRng};

use crate::{
    encoder::Literal,
    kb::{Formula, KnowledgeBase, Var},
    world::{Action, Direction, Perceptions, Position},
};

use agent::{
    problem::{Problem, SuitableState, Utility},
    statexplorer::resolver::AStarExplorer,
};

use agent::problem::CostructSolution;

enum Objective {
    TakeGold,
    GoHome,
}

fn h(p: &Position) -> i32 {
    p.x as i32 + p.y as i32
}

struct FindPlan<'a> {
    dest: Position,
    world_map: &'a HashSet<Position>,
    size_map: usize,
    suitable: fn(Position, Position) -> bool,
}

fn eq_to_dest(_this: Position, _that: Position) -> bool {
    _this == _that
}

impl<'a> FindPlan<'a> {
    fn new(
        dest: Position,
        world_map: &'a HashSet<Position>,
        size_map: usize,
        suitable: fn(Position, Position) -> bool,
    ) -> Self {
        Self {
            dest: dest,
            world_map: world_map,
            size_map: size_map,
            suitable: suitable,
        }
    }
}

impl Problem for FindPlan<'_> {
    type State = Position;
}

impl CostructSolution for FindPlan<'_> {
    type Action = Position;
    type Cost = i32;

    fn executable_actions(&self, state: &Self::State) -> impl Iterator<Item = Self::Action> {
        use Direction::*;

        let mut result = vec![];

        for dir in [North, Sud, East, Ovest] {
            if state.possible_move(dir, self.size_map) {
                let next_pos = state.move_clone(dir);
                if self.world_map.contains(&next_pos) {
                    result.push(next_pos);
                }
            }
        }

        result.into_iter()
    }

    fn result(&self, state: &Self::State, action: &Self::Action) -> (Self::State, Self::Cost) {
        (*action, 1)
    }
}

impl Utility for FindPlan<'_> {
    fn heuristic(&self, state: &Self::State) -> Self::Cost {
        h(state)
    }
}

impl SuitableState for FindPlan<'_> {
    fn is_suitable(&self, state: &Self::State) -> bool {
        (self.suitable)(*state, self.dest)
    }
}

pub struct Hero<K> {
    kb: K,
    obj: Objective,
    t: usize, // time
    visited: HashSet<Position>,
    dangeours: HashSet<Position>,
    safe: HashSet<Position>,
    rng: ThreadRng,
    plan: Option<Vec<Position>>,
    size_map: usize,
}

impl<K> Hero<K> {
    pub fn new(kb: K, size_map: usize) -> Self {
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
            plan: None,
            size_map: size_map,
        }
    }

    fn utility_take_gold(&mut self, a: &Action, p: &Position) -> i32 {
        match *a {
            Action::Move(direction) => {
                if self.visited.contains(&p.move_clone(direction)) {
                    // costruisci un piano che dalla posizione corrente si sposta in una casella safe non ancora visitata
                    // l'utilità di questa mossa sarà la lunghezza del piano negativa

                    // il piano utilizzerà BFS perché non mi viene in mente nessuna euristica consistente per questo problema :(
                    // il costo di una qualsiasi mossa sarà 1, quindi la BFS troverà il piano ottimo

                    // per il principio di ottimalità l'agente continuerà a seguire il path ottimo
                    // anche al prossimo turno

                    // se un piano non esiste allora vuol dire che non possiamo continuare ad esplorare il dungeon
                    // in sicurezza, quindi siamo costretti a cambiare obbiettivo e tornare a casa senza l'oro

                    // Quindi va annullato il piano e va chiamata la funzione utility_go_home e ritornare l'utilità nuova trovata

                    -1
                } else {
                    1
                }
            }
            Action::Grab => i32::MAX,
            Action::Shoot(direction) => todo!(),
            Action::Exit => i32::MIN,
        }
    }

    // ATTENZIONE: il piano potrebbe rimanere null se non ha trovato nessun piano
    fn create_plan(&mut self, actual_position: Position, dest: Position) {
        assert!(self.plan.is_none());

        // crea una frontiera e i nodi esplorati
        let arena = Bump::new();
        let problem = FindPlan::new(dest, &self.visited, self.size_map, eq_to_dest);
        let mut resolver = AStarExplorer::new(&problem, &arena);
        let result = resolver.search(actual_position);
        if let Some(plan) = result.actions.as_ref() {
            println!("[INFO] Plan generated: {:?}", plan);
        } else {
            println!("[WARNING] The hero failed to find a plan");
        }
        self.plan = result.actions;
    }

    fn utility_go_home(&mut self, a: &Action, p: &Position) -> i32 {
        // inizia una ricarca A* per trovare il cammino ottimo per andare dalla posizione
        // fino alla casella (0,0)
        // euristica: distanza manhattan dalla posizione della cella fino al punto (0,0):
        // quindi h(x,y) =(x - 0) + (y - 0) = x + y

        // crea una funzione di utilità che preferisce tutte le mosse che portano
        // dalla posizione corrente fino alla cella (0,0)

        // Sia G il cammino ottimo [n,n',...,n_0] allora la funzione di utilità
        // dovrà dare ad ogni nodo n la seguente utilità:
        // -h(n.x,n.y)
        // dato che l'agente cercarà di massimizzare l'utilità lo porterà alla cella (0,0)

        // G sarà il "piano" dell'agente, se il piano esiste allora usa quello esistente per
        // dare l'utilità alle posizioni
        // se il piano agente non esiste allora creane uno partendo dalla posizione attuale

        // Tutte le altre mosse hanno utilità -inf, tranne dell'azione Exit che avrà utilità +inf

        if self.plan.is_none() {
            self.create_plan(*p, Position::new(0, 0));
            if self.plan.is_none() {
                println!(
                    "[FATAL ERROR] There is always a plan to (0,0) from the actual position, but the hero failed to find it"
                );
            }
        }

        let plan = self.plan.as_ref().expect("The plan was found");

        match *a {
            Action::Move(direction) => {
                let mut found = false;
                let next_pos = p.move_clone(direction);
                for pos in plan {
                    if *pos == next_pos {
                        found = true;
                        break;
                    }
                }
                if found { -h(&next_pos) } else { i32::MIN }
            }
            Action::Grab => i32::MAX,
            Action::Shoot(direction) => i32::MIN,
            Action::Exit => i32::MAX,
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

        if p.position == Position::new(0, 0) {
            suitable_actions.push(Exit);
        }

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
                println!("[INFO] Inferred: {:?}", formula);
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
            println!("[ERROR] no action possible");
            exit(1);
        }
    }
}
