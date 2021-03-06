#![feature(test)]

extern crate fibe;
extern crate test;
extern crate pulse;
extern crate future_pulse;

use fibe::*;
use test::Bencher;
use pulse::Signals;
use future_pulse::Future;

fn warmup(front: &mut fibe::Frontend) {
    for _ in 0..100 {
        task(|_| {}).start(front);
    }
    task(|_| {}).start(front).get();
}

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
            last = task(move |_| {}).after(last.signal()).start(&mut front);
        }
        front.die(fibe::Wait::Pending);
    });
}

#[bench]
fn chain_10_wait(b: &mut Bencher) {
    let mut front = fibe::Frontend::new();
    warmup(&mut front);
    b.iter(|| {
        let mut last = task(move |_| {}).start(&mut front);
        for _ in 1..10 {
            last = task(move |_| {}).after(last.signal()).start(&mut front);
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
            last = task(move |_| {}).after(last.signal()).start(&mut front);
        }
        front.die(fibe::Wait::Pending);
    });
}

#[bench]
fn chain_1_000_wait(b: &mut Bencher) {
    let mut front = fibe::Frontend::new();
    warmup(&mut front);
    b.iter(|| {
        let mut last = task(move |_| {}).start(&mut front);
        for _ in 1..1_000 {
            last = task(move |_| {}).after(last.signal()).start(&mut front);
        }
        last.wait().unwrap();
    });
}


fn fibb_steal(depth: usize, front: &mut fibe::Frontend) -> Future<u64> {
    let task = task(move |_| {1});
    if depth == 0 {
        task
    } else {
        let left = fibb_steal(depth - 1, front);
        let right = fibb_steal(depth - 1, front);
        task.after(left.signal()).after(right.signal())
    }.start(front)
}

#[bench]
fn bench_fibb_steal(b: &mut Bencher) {
    let mut front = fibe::Frontend::new();
    warmup(&mut front);
    b.iter(|| {
        fibb_steal(8, &mut front).wait().unwrap();
    });
}

#[bench]
fn fanout_1_000(b: &mut Bencher) {
    let mut front = fibe::Frontend::new();
    warmup(&mut front);
    b.iter(|| {
        let (signal, pulse) = pulse::Signal::new();
        let signals: Vec<pulse::Signal> = (0..1_000).map(|_|
            task(move |_| {}).after(signal.clone()).start(&mut front).signal()
        ).collect();
        pulse.pulse();
        pulse::Barrier::new(&signals).wait().unwrap();
    });
}

/*
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
*/

#[bench]
fn chain_10_fibers(b: &mut Bencher) {
    let mut front = Frontend::new();
    warmup(&mut front);
    b.iter(|| {
        let (mut s, p) = Future::new();
        for _ in 0..10 {
            s = task(|_| s.get()).start(&mut front);
        }
        p.set(());
        s.get();
    });
}

#[bench]
fn spawn(b: &mut Bencher) {
    let mut front = Frontend::new();
    warmup(&mut front);

    b.iter(|| {
        task(|_| {}).start(&mut front).get();
    });
}

#[bench]
fn spawn_get(b: &mut Bencher) {
    let mut front = Frontend::new();
    warmup(&mut front);

    b.iter(|| {
        task(|_| {}).start(&mut front);
    });
}