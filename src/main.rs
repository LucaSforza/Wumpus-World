mod encoder;
mod hero;
mod kb;
mod world;

use crate::{hero::Hero, kb::init_kb, world::World};

// true se trova l'oro false altrimenti
fn simulate(dim: usize, pit_number: usize) -> bool {
    let mut world = World::new(dim, pit_number);
    let mut hero = Hero::new(init_kb(dim), dim);
    print!("{}", world);
    loop {
        let p = world.perceptions();
        let a = hero.next_action(p);
        let (finish, gold) = world.do_action(a);
        print!("{}", world);
        if finish {
            return gold;
        }
    }
}

fn main() {
    // let dim = 20;
    // let mut world = World::new(dim, 40);
    // let mut hero = Hero::new(init_kb(dim), dim);
    // print!("{}", world);
    // loop {
    //     let p = world.perceptions();
    //     let a = hero.next_action(p);
    //     world.do_action(a);
    //     print!("{}", world);
    // }
    let mut gold_found = 0;
    for _ in 0..100 {
        if simulate(10, 12) {
            gold_found += 1;
        }
    }
    println!("[FINISH] gold found: {} ", (gold_found as f64) / 100.0);
}
