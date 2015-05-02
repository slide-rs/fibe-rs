extern crate fibe;
extern crate timebomb;

use timebomb::timeout_ms;

#[test]
fn die_empty_none() {
    timeout_ms(|| {
        let front = fibe::Frontend::new();
        front.die(fibe::Wait::None);
    }, 3000);
}

#[test]
fn die_empty_pending() {
    timeout_ms(|| {
        let front = fibe::Frontend::new();
        front.die(fibe::Wait::Pending);
    }, 3000);
}

#[test]
fn die_empty_active() {
    timeout_ms(|| {
        let front = fibe::Frontend::new();
        front.die(fibe::Wait::Active);
    }, 3000);
}

#[test]
fn die_all_active_none() {
    timeout_ms(|| {
        let mut front = fibe::Frontend::new();
        for _ in 0..10 {
            front.add(move || {}, vec![]);
        }
        front.die(fibe::Wait::None);
    }, 3000);
}

#[test]
fn die_all_active_pending() {
    timeout_ms(|| {
        let mut front = fibe::Frontend::new();
        for _ in 0..10 {
            front.add(move || {}, vec![]);
        }
        front.die(fibe::Wait::Pending);
    }, 3000);
}

#[test]
fn die_all_active_active() {
    timeout_ms(|| {
        let mut front = fibe::Frontend::new();
        for _ in 0..10 {
            front.add(move || {}, vec![]);
        }
        front.die(fibe::Wait::Active);
    }, 3000);
}

#[test]
fn die_pending_chain_none() {
    timeout_ms(|| {
        let mut front = fibe::Frontend::new();
        let mut last = front.add(move || {}, vec![]);
        for _ in 1..10 {
            last = front.add(move || {}, vec![last]);
        }
        front.die(fibe::Wait::None);
    }, 3000);
}

#[test]
fn die_pending_chain_pending() {
    timeout_ms(|| {
        let mut front = fibe::Frontend::new();
        let mut last = front.add(move || {}, vec![]);
        for _ in 1..10 {
            last = front.add(move || {}, vec![last]);
        }
        front.die(fibe::Wait::Pending);
    }, 3000);
}

#[test]
fn die_pending_chain_active() {
    timeout_ms(|| {
        let mut front = fibe::Frontend::new();
        let mut last = front.add(move || {}, vec![]);
        for _ in 1..10 {
            last = front.add(move || {}, vec![last]);
        }
        front.die(fibe::Wait::Active);
    }, 3000);
}
