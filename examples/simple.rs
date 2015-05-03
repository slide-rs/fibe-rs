extern crate fibe;

use fibe::*;

fn test(wait: Wait) {
    let front = Frontend::new();
    let ha = front.add(move |_| {print!("Hello, ")}, None);
    let hb = front.add(move |_| {println!("world")}, Some(ha));
    let _ = hb;
    front.die(wait);
}

fn main() {
    test(Wait::None);
    test(Wait::Active);
    test(Wait::Pending);
}
