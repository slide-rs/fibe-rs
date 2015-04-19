//! Back-end module for the task queue. The back-end is running
//! on a separate thread. All it does is listening to a command
//! channel and starting new tasks when the time comes.

use std::boxed::FnBox;
use std::collections::HashMap;
use std::sync::Mutex;
use std::thread;
use pulse::{Pulse, Trigger, Select, Barrier};

use {Handle, Wait};


struct Pending {
    task: Box<FnBox() + Send>,
    done: Trigger
}

struct Inner {
    exit: Option<Trigger>,
    exit_method: Wait,

    pending_select: Select,
    pending: HashMap<usize, Pending>,

    active_select: Select,
    active: HashMap<usize, thread::JoinHandle<()>>
}

/// Task queue back-end.
pub struct Backend {
    inner: Mutex<Inner>
}

impl Backend {
    /// Create a new back-end.
    pub fn new() -> Backend {
        Backend {
            inner: Mutex::new(Inner{
                exit: None,
                exit_method: Wait::None,
                pending_select: Select::new(),
                pending: HashMap::new(),
                active_select: Select::new(),
                active: HashMap::new(),
            })
        }
    }

    fn launch(&self, pending: Pending) {
        let (p, t0) = Pulse::new();
        let Pending {
            task,
            done: t1
        } = pending;

        let thread = thread::spawn(move|| {
            (task)();
            t0.trigger();
            t1.trigger();
        });

        let mut guard = self.inner.lock().unwrap();
        let id = guard.active_select.add(p);
        guard.active.insert(id, thread);
    }

    pub fn start(&self, deps: Vec<Handle>, task: Box<FnBox() + Send>) -> Handle {
        let barrier = Barrier::new(deps);
        let pulse = barrier.pulse();

        let (p, t) = Pulse::new();
        let pending = Pending {
            task: task,
            done: t
        };
        if pulse.is_pending() {
            let mut guard = self.inner.lock().unwrap();
            let id = guard.pending_select.add(pulse);
            guard.pending.insert(id, pending);
        } else {
            self.launch(pending);
        }
        p
    }

    pub fn exit(&self, wait: Wait) {
        let mut guard = self.inner.lock().unwrap();
        guard.exit_method = wait;
        let t = guard.exit.take().unwrap();
        t.trigger();
    }

    pub fn run(&self, ack: Trigger) {
        let (exit_p, exit) = Pulse::new();
        let mut select = Select::new();
        let exit_id = select.add(exit_p);
        let (mut pending_id, mut active_id) = {
            let mut guard = self.inner.lock().unwrap();
            guard.exit = Some(exit);
            (select.add(guard.pending_select.pulse()),
             select.add(guard.active_select.pulse()))
        };

        ack.trigger();

        let mut exit_method = None;
        while let Some(pulse) = select.next() {
            if pulse.id() == pending_id {
                let mut guard = self.inner.lock().unwrap();
                pending_id = select.add(guard.pending_select.pulse());
                if let Some(pending) = guard.pending_select.try_next() {
                    let task = guard.pending.remove(&pending.id()).unwrap();
                    drop(guard);
                    if exit_method != Some(Wait::Active) {
                        self.launch(task);
                    }
                }
            } else if pulse.id() == active_id {
                let mut guard = self.inner.lock().unwrap();
                active_id = select.add(guard.active_select.pulse());
                if let Some(active) = guard.active_select.try_next() {
                    let task = guard.active.remove(&active.id()).unwrap();
                    let count = guard.active.len();
                    task.join().unwrap();
                    drop(guard);
                    if count == 0 {
                        match exit_method {
                            Some(Wait::Active) |
                            Some(Wait::Pending) => break,
                            _ => ()
                        }                        
                    }
                };
            } else if exit_id == pulse.id() {
                let guard = self.inner.lock().unwrap();
                exit_method = Some(guard.exit_method);
                if exit_method == Some(Wait::None) {
                    break;
                }
            }
        }
    }
}
