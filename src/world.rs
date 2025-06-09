use std::{fmt, process::exit};

use rand::Rng;

#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
enum Entity {
    Pit,
    Wumpus,
    Gold,
}

#[derive(Clone, PartialEq, Eq)]
pub enum Action {
    Move(Direction),
    Grab,
    Shoot(Direction),
    Exit,
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
pub struct Perceptions {
    pub glitter: bool,
    pub stench: bool,
    pub breeze: bool,
    pub howl: bool,
    pub bump: bool,
    pub position: Position,
    pub board_size: usize,
}

#[derive(Default, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

impl Position {
    pub fn new(x: usize, y: usize) -> Self {
        Self { x: x, y: y }
    }

    pub fn move_clone(&self, dir: Direction) -> Self {
        match dir {
            Direction::North => Self::new(self.x, self.y - 1),
            Direction::Sud => Self::new(self.x, self.y + 1),
            Direction::East => Self::new(self.x + 1, self.y),
            Direction::Ovest => Self::new(self.x - 1, self.y),
        }
    }

    pub fn move_in(&mut self, dir: Direction) {
        match dir {
            Direction::North => self.y -= 1,
            Direction::Sud => self.y += 1,
            Direction::East => self.x += 1,
            Direction::Ovest => self.x -= 1,
        }
    }

    pub fn possible_move(&self, dir: Direction, size: usize) -> bool {
        match dir {
            Direction::North => self.y > 0,
            Direction::Sud => self.y < size - 1,
            Direction::East => self.x < size - 1,
            Direction::Ovest => self.x > 0,
        }
    }
}

pub struct World {
    dungeon: Vec<Vec<Option<Entity>>>,
    gold_in_dungeon: bool,
    hero_pos: Position,
    arrow: bool,
}

impl World {
    pub fn new(dim: usize, pit_number: usize) -> Self {
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
            arrow: true,
            gold_in_dungeon: true,
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

    pub fn perceptions(&self) -> Perceptions {
        let mut p = Perceptions::default();
        p.board_size = self.dungeon.len();
        p.position = self.hero_pos;
        let x = self.hero_pos.x;
        let y = self.hero_pos.y;
        if self.there_is_gold(x, y) {
            p.glitter = true;
        }
        // TODO: compatta
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

    pub fn do_action(&mut self, action: Action) {
        match action {
            Action::Move(dir) => self.hero_pos.move_in(dir),
            Action::Grab => {
                if self.dungeon[self.hero_pos.y][self.hero_pos.x]
                    .as_ref()
                    .map_or(false, |x| *x != Entity::Gold)
                {
                    println!("[FATAL ERROR] The hero is trying to Grap the Gold where is no gold");
                    exit(1)
                }
                self.gold_in_dungeon = false;
                self.dungeon[self.hero_pos.y][self.hero_pos.x] = None
            }
            Action::Shoot(dir) => todo!(),
            Action::Exit => {
                if self.hero_pos == Position::new(0, 0) {
                    if !self.gold_in_dungeon {
                        println!("[SUCCESS] The Hero succesfuly exit the dungeon WITH the gold");
                    } else {
                        println!("[SUCCESS] The Hero succesfuly exit the dungeon WITHOUT the gold")
                    }
                    exit(0);
                } else {
                    println!(
                        "[FATAL ERROR] The agent exited the dangeon in the position: {:?} But he can exit only in the position (0,0)",
                        self.hero_pos
                    );
                    exit(1);
                }
            }
        }
        if self.dungeon[self.hero_pos.y][self.hero_pos.x]
            .as_ref()
            .map(|x| *x == Entity::Wumpus || *x == Entity::Pit)
            .unwrap_or(false)
        {
            println!("{}", self);
            println!("[ERROR] The hero is dead");
            exit(1);
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
pub enum Direction {
    North,
    Sud,
    East,
    Ovest,
}
