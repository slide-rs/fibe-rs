extern crate fibe;
extern crate timebomb;
extern crate pulse;
extern crate future_pulse;

use fibe::*;
use pulse::Signals;
use future_pulse::Future;
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
            task(move |_| {}).start(&mut front);
        }
        front.die(fibe::Wait::None);
    }, 3000);
}

#[test]
fn die_all_active_pending() {
    timeout_ms(|| {
        let mut front = Frontend::new();
        for _ in 0..10 {
            task(move |_| {}).start(&mut front);
        }
        front.die(fibe::Wait::Pending);
    }, 3000);
}

#[test]
fn die_all_active_active() {
    timeout_ms(|| {
        let mut front = Frontend::new();
        for _ in 0..10 {
            task(move |_| {}).start(&mut front);
        }
        front.die(fibe::Wait::Active);
    }, 3000);
}

#[test]
fn die_pending_chain_none() {
    timeout_ms(|| {
        let mut front = Frontend::new();
        let mut last = task(move |_| {}).start(&mut front);
        for _ in 1..10 {
            last = task(move |_| {})
                               .after(last.signal())
                               .start(&mut front);
        }
        front.die(fibe::Wait::None);
    }, 3000);
}

#[test]
fn die_pending_chain_pending() {
    timeout_ms(|| {
        let mut front = Frontend::new();
        let mut last = task(move |_| {}).start(&mut front);
        for _ in 1..10 {
            last = task(move |_| {}).after(last.signal()).start(&mut front);
        }
        front.die(fibe::Wait::Pending);
    }, 3000);
}

#[test]
fn die_pending_chain_active() {
    timeout_ms(|| {
        let mut front = Frontend::new();
        let mut last = task(move |_| {}).start(&mut front);
        for _ in 1..10 {
            last = task(move |_| {})
                               .after(last.signal())
                               .start(&mut front);
        }
        front.die(fibe::Wait::Active);
    }, 3000);
}

#[test]
fn spawn_child() {
    timeout_ms(|| {
        let mut front = Frontend::new();
        let last = task(move |s| {
            let a = task(move |_| {}).start(s);
            let b = task(move |_| {}).start(s);
            a.wait().unwrap();
            b.wait().unwrap();
        }).start(&mut front);
        last.wait().unwrap();
    }, 3000);
}

/*
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
                task(move |_| {}).start(sched)
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
        count.start(&mut front).wait().unwrap();
        rx.try_recv().ok().expect("Task should have sent an ack");
    }, 3000);
}
*/

#[test]
fn fiber_test() {
    timeout_ms(|| {
        let mut front = Frontend::new();
        let (s0, p0) = pulse::Signal::new();
        let (s1, p1) = pulse::Signal::new();
        task(|_| {
            s0.wait().unwrap();
            p1.pulse();
        }).start(&mut front);
        
        assert!(s1.is_pending());
        p0.pulse();
        s1.wait().unwrap();
    }, 3000);
}


#[test]
fn fiber_test_1k() {
    timeout_ms(|| {
        let mut front = Frontend::new();
        let (mut future, set) = Future::new();
        for _ in 0..1_000 {
            future = task(|_| future.get() + 1).start(&mut front);
        }
        set.set(0);
        assert_eq!(future.get(), 1_000);
    }, 3000);
}