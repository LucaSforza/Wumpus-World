use rand::{Rng, rngs::ThreadRng};

use std::{collections::HashSet, fmt, vec};

use crate::encoder::{EncoderSAT, Literal, Literal::Neg, Literal::Pos};

mod encoder;

#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
enum Entity {
    Pit,
    Wumpus,
    Gold,
}
type Dungeon = Vec<Vec<Option<Entity>>>;

fn generate_random_position_not_covered<R: Rng + ?Sized>(
    dungeon: &Dungeon,
    rng: &mut R,
) -> (usize, usize) {
    let dim = dungeon.len();
    let mut x = rng.random_range(0..dim);
    let mut y = rng.random_range(0..dim);
    while (x == 0 && y == 0) || dungeon[y][x].is_some() {
        x = rng.random_range(0..dim);
        y = rng.random_range(0..dim);
    }
    (x, y)
}

#[derive(Default, Debug)]
struct Perceptions {
    glitter: bool,
    stench: bool,
    breeze: bool,
    howl: bool,
    bump: bool,
    position: Position,
    board_size: usize,
}

#[derive(Default, Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct Position {
    x: usize,
    y: usize,
}

impl Position {
    fn new(x: usize, y: usize) -> Self {
        Self { x: x, y: y }
    }

    fn move_clone(&self, dir: Direction) -> Self {
        match dir {
            Direction::North => Self::new(self.x, self.y - 1),
            Direction::Sud => Self::new(self.x, self.y + 1),
            Direction::East => Self::new(self.x + 1, self.y),
            Direction::Ovest => Self::new(self.x - 1, self.y),
        }
    }

    fn move_in(&mut self, dir: Direction) {
        match dir {
            Direction::North => self.y -= 1,
            Direction::Sud => self.y += 1,
            Direction::East => self.x += 1,
            Direction::Ovest => self.x -= 1,
        }
    }

    fn possible_move(&self, dir: Direction, size: usize) -> bool {
        match dir {
            Direction::North => self.y > 0,
            Direction::Sud => self.y < size - 1,
            Direction::East => self.x < size - 1,
            Direction::Ovest => self.x > 0,
        }
    }
}

struct World {
    dungeon: Vec<Vec<Option<Entity>>>,
    hero_pos: Position,
    arrow: bool,
    rng: ThreadRng,
}

impl World {
    fn new(dim: usize, pit_number: usize) -> Self {
        assert!(dim > 0);
        assert!(dim * dim > pit_number + 1 + 1); // the cells needed are pitnumber plus one for the wumpus, one for the gold and one for the hero
        let mut dungeon = vec![vec![None; dim]; dim];
        let mut rng = rand::rng();

        for _ in 0..pit_number {
            let (x, y) = generate_random_position_not_covered(&dungeon, &mut rng);
            dungeon[y][x] = Entity::Pit.into();
        }

        let (x, y) = generate_random_position_not_covered(&dungeon, &mut rng);

        dungeon[y][x] = Some(Entity::Wumpus);

        let (x, y) = generate_random_position_not_covered(&dungeon, &mut rng);

        dungeon[y][x] = Entity::Gold.into();

        World {
            dungeon: dungeon,
            hero_pos: Position { x: 0, y: 0 },
            rng: rng,
            arrow: true,
        }
    }

    fn there_is_something(&self, x: usize, y: usize, entity: Entity) -> bool {
        self.dungeon[y][x]
            .as_ref()
            .map(|e| *e == entity)
            .unwrap_or(false)
    }

    fn there_is_a_pit(&self, x: usize, y: usize) -> bool {
        self.there_is_something(x, y, Entity::Pit)
    }

    fn there_is_the_wumpus(&self, x: usize, y: usize) -> bool {
        self.there_is_something(x, y, Entity::Wumpus)
    }

    fn there_is_gold(&self, x: usize, y: usize) -> bool {
        self.there_is_something(x, y, Entity::Gold)
    }

    fn perceptions(&self) -> Perceptions {
        let mut p = Perceptions::default();
        p.board_size = self.dungeon.len();
        p.position = self.hero_pos;
        let x = self.hero_pos.x;
        let y = self.hero_pos.y;
        if self.there_is_gold(x, y) {
            p.glitter = true;
        }
        if self.hero_pos.x != 0 {
            // controlla se ci sta qualcosa a sinistra
            if self.there_is_a_pit(x - 1, y) {
                p.breeze = true;
            } else if self.there_is_the_wumpus(x - 1, y) {
                p.stench = true;
            }
        }
        if self.hero_pos.y != 0 {
            // controlla se ci sta qualcosa in alto
            if self.there_is_a_pit(x, y - 1) {
                p.breeze = true;
            } else if self.there_is_the_wumpus(x, y - 1) {
                p.stench = true;
            }
        }
        if self.hero_pos.x != self.dungeon.len() - 1 {
            // controlla se c'è qualcosa a destra
            if self.there_is_a_pit(x + 1, y) {
                p.breeze = true;
            } else if self.there_is_the_wumpus(x + 1, y) {
                p.stench = true;
            }
        }
        if self.hero_pos.y != self.dungeon.len() - 1 {
            // controlla se c'è qualcosa in basso
            if self.there_is_a_pit(x, y + 1) {
                p.breeze = true;
            } else if self.there_is_the_wumpus(x, y + 1) {
                p.stench = true;
            }
        }
        p
    }

    fn do_action(&mut self, action: Action) {
        match action {
            Action::Move(dir) => self.hero_pos.move_in(dir),
            Action::Grab => self.dungeon[self.hero_pos.y][self.hero_pos.x] = None,
            Action::Shoot(dir) => todo!(),
        }
        if self.dungeon[self.hero_pos.y][self.hero_pos.x]
            .as_ref()
            .map(|x| *x == Entity::Wumpus || *x == Entity::Pit)
            .unwrap_or(false)
        {
            panic!("The hero is dead");
        }
    }
}

impl fmt::Display for World {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (y, row) in self.dungeon.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                if x == self.hero_pos.x && y == self.hero_pos.y {
                    write!(f, "x ")?;
                } else if let Some(e) = cell {
                    match e {
                        Entity::Pit => write!(f, "o ")?,
                        Entity::Wumpus => write!(f, "w ")?,
                        Entity::Gold => write!(f, "g ")?,
                    }
                } else {
                    write!(f, ". ")?;
                }
            }
            writeln!(f)?;
        }
        writeln!(f, "arrow: {}", self.arrow)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
enum Direction {
    North,
    Sud,
    East,
    Ovest,
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
enum Var {
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

type Formula = Vec<Vec<Literal<Var>>>;

trait KnowledgeBase {
    // @return true iff KB |= formula
    fn ask(&self, formula: Formula) -> bool;
    fn tell(&mut self, formula: Formula);
}

impl KnowledgeBase for EncoderSAT<Var> {
    fn ask(&self, formula: Formula) -> bool {
        let mut dual = self.clone();
        // TODO: se la formula ha solo una clausola la sostituzione di tseytin si può risparmiare
        let mut tseytin_clause = vec![];
        for clause in formula {
            // crea una variabile di tseitin t per clausola
            // aggiungi alla KB la clausola (t or clausola)
            // siano alpha_1 or alpha_2 or ... or alpha_k i letterali della clausola
            // aggiungi alla KB le clausole (not t or not alpha_1) and ... and (not t or not alpha_k)
            // aggiungi la clausola (t_1 or t_2 or ... or t_n) dove n è il numero di clausole.
            let tseytin = dual.create_raw_variable();
            tseytin_clause.push(tseytin.clone());
            for literal in &clause {
                let not_literal = dual.register_literal(literal.not());
                let not_tseytin = tseytin.not();
                dual.add_raw_clause(vec![not_literal, not_tseytin]);
            }
            let mut raw_clause = dual.register_clause(clause);
            raw_clause.push(tseytin.clone());
            dual.add_raw_clause(raw_clause); // aggiunta clausola t or clausola
        }
        dual.add_raw_clause(tseytin_clause);
        !dual.picosat_sat() // TODO: generalize for all the solvers
    }

    fn tell(&mut self, formula: Formula) {
        for clause in formula {
            self.add(clause);
        }
    }
}

fn init_kb(size: usize) -> EncoderSAT<Var> {
    use Var::*;

    let mut kb = EncoderSAT::new();

    // il wumpus esiste in almeno una posizione

    let mut clause = kb.clause();

    for i in 0..size {
        for j in 0..size {
            clause.add(Wumpus {
                pos: Position { x: i, y: j },
            });
            println!("i,j: {:?}", (i, j));
        }
    }
    kb = clause.end();

    // la stanza 0 0 è sicura
    clause = kb.clause();
    clause.add(Safe {
        pos: Position::new(0, 0),
    });
    kb = clause.end();

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

    // in una stanza c'è vento se e solo se in una stanza adiacente c'è il pozzo
    let mut vento_implica_pozzi = vec![];
    // let mut pozzo_implica_vento = vec![];
    // in una stanza c'è puzza se e solo se in una stanza adiacente c'è il Wumpus
    let mut puzza_implica_wumpus = vec![];
    // let mut wumpus_implica_puzza = vec![];

    use Direction::*;

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

    // se una casella è safe allora non c'è il wumpus e non c'è il pozzo
    // se in una casella non c'è il wumpus allora è safe
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

    // se il wumpus ha urlato, allora la cella dove stava il wumpus è sicura
    println!("{:?}", kb);
    // se ha sentito il rumore della freccia sbattere, allora in tutte le celle in cui è passata la freccia non ci sta il wumpus
    kb
}

#[derive(Clone, PartialEq, Eq)]
enum Action {
    Move(Direction),
    Grab,
    Shoot(Direction),
}

struct Hero {
    kb: Box<dyn KnowledgeBase>,
    t: usize, // time
    visited: HashSet<Position>,
    rng: ThreadRng,
}

impl Hero {
    fn new(kb: Box<dyn KnowledgeBase>) -> Self {
        Self {
            kb: kb,
            t: 0,
            visited: HashSet::new(),
            rng: rand::rng(),
        }
    }

    fn create_formula_perception(&self, p: &Perceptions) -> Formula {
        use Var::*;

        let mut formula = Vec::new();
        let mut var: Literal<Var> = Breeze { pos: p.position }.into();
        if !p.breeze {
            var = var.not();
        }
        formula.push(vec![var]);
        var = Gold { pos: p.position }.into();
        if !p.glitter {
            var = var.not();
        }
        formula.push(vec![var]);
        var = Stench { pos: p.position }.into();
        if !p.stench {
            var = var.not();
        }
        formula.push(vec![var]);

        // TODO: bump and howl

        formula
    }

    fn create_formula_ask(&self, a: &Action, pos: &Position) -> Formula {
        use Var::*;

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

    fn next_action(&mut self, p: Perceptions) -> Action {
        use Action::*;
        use Direction::*;

        println!("{:?}", p);

        self.kb.tell(self.create_formula_perception(&p));

        let mut action_to_consider = Vec::with_capacity(9);

        for dir in [North, Sud, East, Ovest] {
            if p.position.possible_move(dir, p.board_size) {
                action_to_consider.push(Move(dir));
            }
        }

        if p.glitter {
            action_to_consider.push(Grab);
        }

        // TODO: add arrow

        let mut suitable_actions = vec![];

        for a in action_to_consider {
            if self.kb.ask(self.create_formula_ask(&a, &p.position)) {
                suitable_actions.push(a);
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

fn main() {
    let dim = 4;
    let mut world = World::new(dim, 1);
    let mut hero = Hero::new(Box::new(init_kb(dim)));
    print!("{}", world);
    for _ in 0..100 {
        let p = world.perceptions();
        let a = hero.next_action(p);
        world.do_action(a);
        print!("{}", world);
    }
}
