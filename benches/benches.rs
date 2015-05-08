#![feature(test)]

extern crate fibe;
extern crate test;
extern crate pulse;

use fibe::*;
use test::Bencher;
use pulse::Signals;

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

fn fibb_steal(depth: usize, front: &mut fibe::Frontend) -> fibe::Handle {
    let task = task(move |_| {});
    if depth == 0 {
        task
    } else {
        let left = fibb_steal(depth - 1, front);
        let right = fibb_steal(depth - 1, front);
        task.after(left).after(right)
    }.start(front)
}

#[bench]
fn bench_fibb_steal(b: &mut Bencher) {
    let mut front = fibe::Frontend::new();
    b.iter(|| {
        fibb_steal(8, &mut front).wait().unwrap();
    });
}

#[bench]
fn fanout_1_000(b: &mut Bencher) {
    let mut front = fibe::Frontend::new();
    b.iter(|| {
        let (signal, pulse) = pulse::Signal::new();
        let signals: Vec<pulse::Signal> = (0..1_000).map(|_|
            task(move |_| {}).after(signal.clone()).start(&mut front)
        ).collect();
        pulse.pulse();
        pulse::Barrier::new(&signals).wait().unwrap();
    });
}

struct Repeater(usize);

impl ResumableTask for Repeater {
    #[inline(never)]
    fn resume(&mut self, _: &mut Schedule) -> WaitState {
        self.0 -= 1;
        if self.0 == 0 {
            WaitState::Completed
        } else {
            WaitState::Pending(pulse::Signal::pulsed())
        }
    }
}

#[bench]
fn repeat_1_000(b: &mut Bencher) {
    let mut front = fibe::Frontend::new();
    b.iter(|| {
        Repeater(1_000).start(&mut front).wait().unwrap();
    });   
}

#[bench]
fn repeat_100_x_100(b: &mut Bencher) {
    let mut front = fibe::Frontend::new();
    b.iter(|| {
        let signals: Vec<pulse::Signal> = (0..100).map(|_|
            Repeater(100).start(&mut front)
        ).collect();
        pulse::Barrier::new(&signals).wait().unwrap();
    });   
}

#[bench]
fn repeat_1_000_x_1_000(b: &mut Bencher) {
    let mut front = fibe::Frontend::new();
    b.iter(|| {
        let signals: Vec<pulse::Signal> = (0..1_000).map(|_|
            Repeater(1_000).start(&mut front)
        ).collect();
        pulse::Barrier::new(&signals).wait().unwrap();
    });   
}