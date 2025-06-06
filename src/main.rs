use rand::{Rng, rngs::ThreadRng};

use std::fmt;

use crate::encoder::Literal;

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

#[derive(Default)]
struct Perceptions {
    glitter: bool,
    stench: bool,
    breeze: bool,
    howl: bool,
    bump: bool,
}

struct Pos {
    x: usize,
    y: usize,
}

struct World {
    dungeon: Vec<Vec<Option<Entity>>>,
    hero_pos: Pos,
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
            hero_pos: Pos { x: 0, y: 0 },
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
            Action::MoveUp => todo!(),
            Action::MoveDown => todo!(),
            Action::MoveLeft => todo!(),
            Action::MoveRight => todo!(),
            Action::Grab => todo!(),
            Action::Shoot(dir) => todo!(),
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

enum Direction {
    North,
    Sud,
    East,
    Ovest,
}

enum Var {
    Wumpus { x: usize, y: usize },
    Pit { x: usize, y: usize },
    Glitter { x: usize, y: usize },
    Stench { x: usize, y: usize },
    Breeze { x: usize, y: usize },
    Howl,
    Bump { x: usize, y: usize, dir: Direction },
}

type Formula = Vec<Vec<Literal<Var>>>;

trait KnowledgeBase {
    fn ask(&self, formula: Formula) -> Action;
    fn tell(&self, formula: Formula);
}

enum Action {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    Grab,
    Shoot(Direction),
}

struct Hero {
    kb: Box<dyn KnowledgeBase>,
    t: usize, // time
}

impl Hero {
    fn create_formula_perception(&self, p: &Perceptions) -> Formula {
        todo!()
    }

    fn create_formula_tell(&self) -> Formula {
        todo!()
    }

    fn create_action_formula(&self, a: &Action) -> Formula {
        todo!()
    }

    fn next_action(&mut self, p: Perceptions) -> Action {
        self.kb.tell(self.create_formula_perception(&p));
        let action = self.kb.ask(self.create_formula_tell());
        self.kb.tell(self.create_action_formula(&action));
        self.t += 1;
        action
    }
}

fn main() {
    let world = World::new(5, 3);
    print!("{}", world);
}
