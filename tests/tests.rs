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
        let mut front = Frontend::new();
        for _ in 0..10 {
            TaskBuilder::func(move |_| {}).start(&mut front);
        }
        front.die(fibe::Wait::None);
    }, 3000);
}

#[test]
fn die_all_active_pending() {
    timeout_ms(|| {
        let mut front = Frontend::new();
        for _ in 0..10 {
            TaskBuilder::func(move |_| {}).start(&mut front);
        }
        front.die(fibe::Wait::Pending);
    }, 3000);
}

#[test]
fn die_all_active_active() {
    timeout_ms(|| {
        let mut front = Frontend::new();
        for _ in 0..10 {
            TaskBuilder::func(move |_| {}).start(&mut front);
        }
        front.die(fibe::Wait::Active);
    }, 3000);
}

#[test]
fn die_pending_chain_none() {
    timeout_ms(|| {
        let mut front = Frontend::new();
        let mut last = TaskBuilder::func(move |_| {}).start(&mut front);
        for _ in 1..10 {
            last = TaskBuilder::func(move |_| {})
                               .after(last)
                               .start(&mut front);
        }
        front.die(fibe::Wait::None);
    }, 3000);
}

#[test]
fn die_pending_chain_pending() {
    timeout_ms(|| {
        let mut front = Frontend::new();
        let mut last = TaskBuilder::func(move |_| {}).start(&mut front);
        for _ in 1..10 {
            last = TaskBuilder::func(move |_| {})
                               .after(last)
                               .start(&mut front);
        }
        front.die(fibe::Wait::Pending);
    }, 3000);
}

#[test]
fn die_pending_chain_active() {
    timeout_ms(|| {
        let mut front = Frontend::new();
        let mut last = TaskBuilder::func(move |_| {}).start(&mut front);
        for _ in 1..10 {
            last = TaskBuilder::func(move |_| {})
                               .after(last)
                               .start(&mut front);
        }
        front.die(fibe::Wait::Active);
    }, 3000);
}

#[test]
fn spawn_child() {
    timeout_ms(|| {
        let mut front = Frontend::new();
        let last = TaskBuilder::func(move |s| {
            let a = TaskBuilder::func(move |_| {}).start(s);
            let b = TaskBuilder::func(move |_| {}).start(s);
            a.wait().unwrap();
            b.wait().unwrap();
        }).start(&mut front);
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
            WaitState::Pending(
                TaskBuilder::func(move |_| {}).start(sched)
            )
        }
    }
}

#[test]
fn resumeable_task() {
    timeout_ms(|| {
        let mut front = Frontend::new();
        let (tx, rx) = channel();
        let count = CountDown(1000, tx);
        TaskBuilder::new(count).start(&mut front).wait().unwrap();
        rx.try_recv().ok().expect("Task should have sent an ack");
    }, 3000);
}