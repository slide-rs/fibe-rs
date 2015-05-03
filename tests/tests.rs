extern crate fibe;
extern crate timebomb;

use fibe::*;
use timebomb::timeout_ms;

#[test]
fn die_empty_none() {
    timeout_ms(|| {
        let front = Frontend::new();
        front.die(fibe::Wait::None);
    }, 3000);
}

#[test]
fn die_empty_pending() {
    timeout_ms(|| {
        let front = Frontend::new();
        front.die(fibe::Wait::Pending);
    }, 3000);
}

#[test]
fn die_empty_active() {
    timeout_ms(|| {
        let front = Frontend::new();
        front.die(fibe::Wait::Active);
    }, 3000);
}

#[test]
fn die_all_active_none() {
    timeout_ms(|| {
        let front = Frontend::new();
        for _ in 0..10 {
            front.add(move |_| {}, None);
        }
        front.die(fibe::Wait::None);
    }, 3000);
}

#[test]
fn die_all_active_pending() {
    timeout_ms(|| {
        let front = Frontend::new();
        for _ in 0..10 {
            front.add(move |_| {}, None);
        }
        front.die(fibe::Wait::Pending);
    }, 3000);
}

#[test]
fn die_all_active_active() {
    timeout_ms(|| {
        let front = Frontend::new();
        for _ in 0..10 {
            front.add(move |_| {}, None);
        }
        front.die(fibe::Wait::Active);
    }, 3000);
}

#[test]
fn die_pending_chain_none() {
    timeout_ms(|| {
        let front = Frontend::new();
        let mut last = front.add(move |_| {}, None);
        for _ in 1..10 {
            last = front.add(move |_| {}, Some(last));
        }
        front.die(fibe::Wait::None);
    }, 3000);
}

#[test]
fn die_pending_chain_pending() {
    timeout_ms(|| {
        let front = Frontend::new();
        let mut last = front.add(move |_| {}, None);
        for _ in 1..10 {
            last = front.add(move |_| {}, Some(last));
        }
        front.die(fibe::Wait::Pending);
    }, 3000);
}

#[test]
fn die_pending_chain_active() {
    timeout_ms(|| {
        let front = Frontend::new();
        let mut last = front.add(move |_| {}, None);
        for _ in 1..10 {
            last = front.add(move |_| {}, Some(last));
        }
        front.die(fibe::Wait::Active);
    }, 3000);
}

#[test]
fn spawn_child() {
    timeout_ms(|| {
        let front = Frontend::new();
        let last = front.add(move |s| {
            let a = s.add(move |_| {}, None);
            let b = s.add(move |_| {}, None);
            a.wait().unwrap();
            b.wait().unwrap();
        }, None);
        last.wait().unwrap();
    }, 3000);
}