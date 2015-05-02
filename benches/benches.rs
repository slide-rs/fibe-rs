#![feature(test)]

extern crate fibe;
extern crate test;
extern crate pulse;

use pulse::{Barrier, Signals};
use test::Bencher;

#[bench]
fn start_die(b: &mut Bencher) {
    b.iter(|| {
        let front = fibe::Frontend::new();
        front.die(fibe::Wait::None);
    });
}

#[bench]
fn chain_10_use_die(b: &mut Bencher) {
    b.iter(|| {
        let mut front = fibe::Frontend::new();
        let mut last = front.add(move || {}, None);
        for _ in 1..10 {
            last = front.add(move || {}, Some(last));
        }
        front.die(fibe::Wait::Pending);
    });
}

#[bench]
fn chain_10_wait(b: &mut Bencher) {
    let mut front = fibe::Frontend::new();
    b.iter(|| {
        let mut last = front.add(move || {}, None);
        for _ in 1..10 {
            last = front.add(move || {}, Some(last));
        }
        last.wait().unwrap();
    });
}

#[bench]
fn chain_1_000_use_die(b: &mut Bencher) {
    b.iter(|| {
        let mut front = fibe::Frontend::new();
        let mut last = front.add(move || {}, None);
        for _ in 1..1_000 {
            last = front.add(move || {}, Some(last));
        }
        front.die(fibe::Wait::Pending);
    });
}

#[bench]
fn chain_1_000_wait(b: &mut Bencher) {
    let mut front = fibe::Frontend::new();
    b.iter(|| {
        let mut last = front.add(move || {}, None);
        for _ in 1..1_000 {
            last = front.add(move || {}, Some(last));
        }
        last.wait().unwrap();
    });
}

fn fibb(depth: usize, front: &mut fibe::Frontend) -> fibe::Handle {
    if depth == 0 {
        front.add(move || {}, None)
    } else {
        let left = fibb(depth - 1, front);
        let right = fibb(depth - 1, front);
        front.add(move || {}, Some(Barrier::new(&[left, right]).signal()))
    }
}

#[bench]
fn fibb_depth_6(b: &mut Bencher) {
    b.iter(|| {
        let mut front = fibe::Frontend::new();
        fibb(6, &mut front);
        front.die(fibe::Wait::Pending);
    });
}