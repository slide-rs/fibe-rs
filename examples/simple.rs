extern crate fibe;

fn test(wait: fibe::Wait) {
    let mut front = fibe::Frontend::new();
    let ha = front.add(move || {print!("Hello, ")}, None);
    let hb = front.add(move || {println!("world")}, Some(ha));
    let _ = hb;
    front.die(wait);
}

fn main() {
    test(fibe::Wait::None);
    test(fibe::Wait::Active);
    test(fibe::Wait::Pending);
}
