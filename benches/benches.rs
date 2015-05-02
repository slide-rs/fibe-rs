#![feature(test)]

extern crate fibe;
extern crate test;

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
        let mut last = front.add(move || {}, vec![]);
        for _ in 1..10 {
            last = front.add(move || {}, vec![last]);
        }
        front.die(fibe::Wait::Pending);
    });
}

#[bench]
fn chain_10_wait(b: &mut Bencher) {
    let mut front = fibe::Frontend::new();
    b.iter(|| {
        let mut last = front.add(move || {}, vec![]);
        for _ in 1..10 {
            last = front.add(move || {}, vec![last]);
        }
        last.wait().unwrap();
    });
}

#[bench]
fn chain_1_000_use_die(b: &mut Bencher) {
    b.iter(|| {
        let mut front = fibe::Frontend::new();
        let mut last = front.add(move || {}, vec![]);
        for _ in 1..1_000 {
            last = front.add(move || {}, vec![last]);
        }
        front.die(fibe::Wait::Pending);
    });
}

#[bench]
fn chain_1_000_wait(b: &mut Bencher) {
    let mut front = fibe::Frontend::new();
    b.iter(|| {
        let mut last = front.add(move || {}, vec![]);
        for _ in 1..1_000 {
            last = front.add(move || {}, vec![last]);
        }
        last.wait().unwrap();
    });
}

fn fibb(depth: usize, front: &mut fibe::Frontend) -> fibe::Handle {
    if depth == 0 {
        front.add(move || {}, vec![])
    } else {
        let left = fibb(depth - 1, front);
        let right = fibb(depth - 1, front);
        front.add(move || {}, vec![left, right])
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