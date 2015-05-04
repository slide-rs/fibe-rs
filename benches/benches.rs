#![feature(test)]

extern crate fibe;
extern crate test;
extern crate pulse;

use fibe::*;
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
        let mut last = task(move |_| {}).start(&mut front);
        for _ in 1..10 {
            last = task(move |_| {}).after(last).start(&mut front);
        }
        front.die(fibe::Wait::Pending);
    });
}

#[bench]
fn chain_10_wait(b: &mut Bencher) {
    let mut front = fibe::Frontend::new();
    b.iter(|| {
        let mut last = task(move |_| {}).start(&mut front);
        for _ in 1..10 {
            last = task(move |_| {}).after(last).start(&mut front);
        }
        last.wait().unwrap();
    });
}

#[bench]
fn chain_1_000_use_die(b: &mut Bencher) {
    b.iter(|| {
        let mut front = fibe::Frontend::new();
        let mut last = task(move |_| {}).start(&mut front);
        for _ in 1..1_000 {
            last = task(move |_| {}).after(last).start(&mut front);
        }
        front.die(fibe::Wait::Pending);
    });
}

#[bench]
fn chain_1_000_wait(b: &mut Bencher) {
    let mut front = fibe::Frontend::new();
    b.iter(|| {
        let mut last = task(move |_| {}).start(&mut front);
        for _ in 1..1_000 {
            last = task(move |_| {}).after(last).start(&mut front);
        }
        last.wait().unwrap();
    });
}

fn fibb(depth: usize, front: &mut fibe::Frontend) -> fibe::Handle {
    let task = task(move |_| {});
    if depth == 0 {
        task
    } else {
        let left = fibb(depth - 1, front);
        let right = fibb(depth - 1, front);
        task.after(left).after(right)
    }.start(front)
}

#[bench]
fn fibb_depth_6(b: &mut Bencher) {
    b.iter(|| {
        let mut front = fibe::Frontend::new();
        fibb(6, &mut front);
        front.die(fibe::Wait::Pending);
    });
}