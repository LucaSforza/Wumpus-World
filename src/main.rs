mod encoder;
mod hero;
mod kb;
mod world;

use crate::{hero::Hero, kb::init_kb, world::World};

fn main() {
    let dim = 5;
    let mut world = World::new(dim, 4);
    let mut hero = Hero::new(Box::new(init_kb(dim)));
    print!("{}", world);
    for _ in 0..100 {
        let p = world.perceptions();
        let a = hero.next_action(p);
        world.do_action(a);
        print!("{}", world);
    }
}
