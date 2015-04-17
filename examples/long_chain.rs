extern crate fibe;

fn test(wait: fibe::Wait) {
    let mut front = fibe::Frontend::new();
    let mut last = front.add(move || {}, vec![]);
    for i in 1..300000 {
    	last = front.add(move || {}, vec![last]);
	}
    front.die(wait);
}

fn main() {
    test(fibe::Wait::Pending);
}
