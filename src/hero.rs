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
    statexplorer::resolver::{AStarExplorer, BFSExplorer},
};

use agent::problem::CostructSolution;

#[derive(Default)]
struct Cache {
    visited: HashSet<Position>,
    safe: HashSet<Position>,
    _unsafe: HashSet<Position>,
    wumpus: Option<Position>,
}

impl Cache {
    fn new() -> Self {
        let mut safe = HashSet::new();
        safe.insert(Position::new(0, 0));
        Self {
            safe: safe,
            visited: Default::default(),
            _unsafe: Default::default(),
            wumpus: Default::default(),
        }
    }

    fn is_safe(&self, p: &Position) -> bool {
        self.safe.contains(p)
    }

    fn is_unsafe(&self, p: &Position) -> bool {
        self._unsafe.contains(p)
    }

    fn is_visited(&self, p: &Position) -> bool {
        self.visited.contains(p)
    }

    fn there_is_the_wumpus(&self, p: &Position) -> bool {
        self.is_unsafe(p) && self.wumpus.map_or(false, |x| x == *p)
    }

    fn safe_but_not_visited(&self, p: &Position) -> bool {
        self.is_safe(p) && !self.is_visited(p)
    }
}

enum Objective {
    TakeGold,
    GoHome,
}

fn distance_to_zero(p: &Position) -> i32 {
    p.x as i32 + p.y as i32
}

fn no_heuristic(_p: &Position) -> i32 {
    1
}

struct FindPlan<'a> {
    cache: &'a Cache,
    size_map: usize,
    suitable: fn(&Cache, &Position) -> bool,
    heuristic: fn(&Position) -> i32,
}

fn eq_to_zero(_cache: &Cache, _this: &Position) -> bool {
    *_this == Position::new(0, 0)
}

impl<'a> FindPlan<'a> {
    fn new(
        cache: &'a Cache,
        size_map: usize,
        suitable: fn(&Cache, &Position) -> bool,
        heuristic: fn(&Position) -> i32,
    ) -> Self {
        Self {
            cache: cache,
            size_map: size_map,
            suitable: suitable,
            heuristic: heuristic,
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
                if self.cache.is_safe(&next_pos) {
                    result.push(next_pos);
                }
            }
        }

        result.into_iter()
    }

    fn result(&self, _state: &Self::State, action: &Self::Action) -> (Self::State, Self::Cost) {
        (*action, 1)
    }
}

impl Utility for FindPlan<'_> {
    fn heuristic(&self, state: &Self::State) -> Self::Cost {
        (self.heuristic)(state)
    }
}

impl SuitableState for FindPlan<'_> {
    fn is_suitable(&self, state: &Self::State) -> bool {
        (self.suitable)(self.cache, state)
    }
}

pub struct Hero<K> {
    kb: K,
    obj: Objective,
    t: usize, // time
    cache: Cache,
    rng: ThreadRng,
    plan: Option<Vec<Position>>,
    size_map: usize,
}

impl<K> Hero<K> {
    pub fn new(kb: K, size_map: usize) -> Self {
        Self {
            kb: kb,
            t: 0,
            cache: Cache::new(),
            rng: rand::rng(),
            obj: Objective::TakeGold,
            plan: None,
            size_map: size_map,
        }
    }

    fn utility_take_gold(&mut self, a: &Action, p: &Position) -> i32 {
        match *a {
            Action::Move(direction) => {
                if self.cache.is_visited(&p.move_clone(direction)) {
                    // costruisci un piano che dalla posizione corrente si sposta in una casella safe non ancora visitata
                    // l'utilità di questa mossa sarà la lunghezza del piano negativa

                    // il piano utilizzerà BFS perché non mi viene in mente nessuna euristica consistente per questo problema :(
                    // il costo di una qualsiasi mossa sarà 1, quindi la BFS troverà il piano ottimo

                    // per il principio di ottimalità l'agente continuerà a seguire il path ottimo
                    // anche al prossimo turno

                    // se un piano non esiste allora vuol dire che non possiamo continuare ad esplorare il dungeon
                    // in sicurezza, quindi siamo costretti a cambiare obbiettivo e tornare a casa senza l'oro

                    // Quindi va annullato il piano e va chiamata la funzione utility_go_home e ritornare l'utilità nuova trovata

                    if let Some(plan) = self.plan.clone() {
                        let pos = p.move_clone(direction);
                        // let mut final_pos = false;
                        for (i, pos2) in plan.iter().enumerate() {
                            if *pos2 == pos {
                                if i == plan.len() - 1 {
                                    self.plan = None;
                                }
                                return -((plan.len() - i - 1) as i32);
                            }
                        }
                        return i32::MIN;
                    } else {
                        panic!("There is no plan")
                    }
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
    fn create_plan_to_go_home(&mut self, actual_position: Position) {
        assert!(self.plan.is_none());

        // crea una frontiera e i nodi esplorati
        let arena = Bump::new();
        let problem = FindPlan::new(&self.cache, self.size_map, eq_to_zero, distance_to_zero);
        let mut resolver = AStarExplorer::new(&problem, &arena);
        let result = resolver.search(actual_position);
        if let Some(plan) = result.actions.as_ref() {
            println!("[INFO] Plan generated: {:?}", plan);
        } else {
            println!("[WARNING] The hero failed to find a plan");
        }
        self.plan = result.actions;
    }

    fn create_plan_gold(&mut self, actual_position: Position) {
        assert!(self.plan.is_none());

        // crea una frontiera e i nodi esplorati
        let arena = Bump::new();
        let problem = FindPlan::new(
            &self.cache,
            self.size_map,
            Cache::safe_but_not_visited,
            no_heuristic,
        );
        let mut resolver = BFSExplorer::new(&problem, &arena);
        let result = resolver.search(actual_position);
        if let Some(plan) = result.actions.as_ref() {
            println!("[INFO] Plan generated: {:?}", plan);
        } else {
            println!("[WARNING] The hero failed to find a plan");
        }
        self.plan = result.actions;
    }

    // true se il piano è stato creato, false altrimenti
    fn create_plan(&mut self, actual_position: Position) -> bool {
        match self.obj {
            Objective::TakeGold => self.create_plan_gold(actual_position),
            Objective::GoHome => self.create_plan_to_go_home(actual_position),
        };
        self.plan.is_some()
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

        assert!(self.plan.is_some());

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
                if found {
                    -distance_to_zero(&next_pos)
                } else {
                    i32::MIN
                }
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
                && !self.cache.is_unsafe(&p.position.move_clone(dir))
            {
                if self.cache.is_safe(&p.position.move_clone(dir)) {
                    suitable_actions.push(Move(dir));
                } else {
                    action_to_consider.push(Move(dir));
                }
            }
        }

        if p.glitter {
            action_to_consider.push(Grab);
            self.obj = Objective::GoHome;
            println!("[INFO] Changed Plan,found gold,  go home");
        }

        // TODO: add arrow

        for a in action_to_consider {
            let formula = K::create_query_from_action(&a, &p.position);
            if self.kb.ask(&formula) {
                println!("[INFO] Inferred: {:?}", formula);
                suitable_actions.push(a);
                self.kb.tell(&formula);
                for pos in self.kb.safe_positions(formula).into_iter() {
                    self.cache.safe.insert(pos);
                }
            } else {
                match a {
                    Move(dir) => {
                        if self.kb.is_unsafe(p.position.move_clone(dir)) {
                            self.cache._unsafe.insert(p.position.move_clone(dir));
                        }
                    }
                    _ => {}
                }
            }
        }

        self.cache.visited.insert(p.position);
        if self.plan.as_ref().map_or(true, |x| x.is_empty()) {
            self.plan = None;
            if !self.create_plan(p.position) {
                self.obj = Objective::GoHome;
                println!("[INFO] Changed Plan, go home");
                assert!(self.create_plan(p.position))
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
            return a;
        } else {
            println!("[ERROR] no action possible");
            exit(1);
        }
    }
}
