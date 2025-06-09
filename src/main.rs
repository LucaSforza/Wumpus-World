mod encoder;
mod hero;
mod kb;
mod world;

use crate::{hero::Hero, kb::init_kb, world::World};

fn main() {
    let dim = 10;
    let mut world = World::new(dim, 6);
    let mut hero = Hero::new(init_kb(dim), dim);
    print!("{}", world);
    loop {
        let p = world.perceptions();
        let a = hero.next_action(p);
        world.do_action(a);
        print!("{}", world);
    }
}
