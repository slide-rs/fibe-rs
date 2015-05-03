extern crate fibe;
extern crate timebomb;

use fibe::*;
use timebomb::timeout_ms;
use std::sync::mpsc::{Sender, channel};

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

struct CountDown(u32, Sender<()>);

impl Drop for CountDown {
    fn drop(&mut self) {
        self.1.send(()).unwrap();
        assert!(self.0 == 0);
    }
}

impl ResumableTask for CountDown {
    fn resume(&mut self, sched: &mut Schedule) -> WaitState {
        if self.0 == 0 {
            WaitState::Completed
        } else {
            self.0 -= 1;
            WaitState::Pending(sched.add(move |_| {}, None))
        }
    }
}

#[test]
fn resumeable_task() {
    timeout_ms(|| {
        let front = Frontend::new();
        let (tx, rx) = channel();
        let count = Box::new(CountDown(1000, tx));
        front.add_task(count, None).wait().unwrap();
        rx.try_recv().ok().expect("Task should have sent an ack");
    }, 3000);
}