//! Back-end module for the task queue. The back-end is running
//! on a separate thread. All it does is listening to a command
//! channel and starting new tasks when the time comes.

use std::boxed::FnBox;
use std::thread;
use std::sync::atomic::*;
use std::sync::Arc;
use atom::*;
use pulse::*;

use {Handle, Wait};

// Todo 64bit verison
const BLOCK: usize = 0x8000_0000;
const REF_COUNT: usize = 0x7FFF_FFFF;

struct Inner {
    active: AtomicUsize,
    work_done: Atom<Pulse>
}

impl Inner {
    /// Check to see if the scheduler has put a hold on the
    /// starting of new tasks (occurs during shutdown)
    fn try_active_inc(&self) -> bool {
        loop {
            let value = self.active.load(Ordering::SeqCst);
            if value & BLOCK == BLOCK {
                return false;
            }

            // This is used instead of a fetch_add to allow for checking of the
            // block flag
            if value == self.active.compare_and_swap(value, value + 1, Ordering::SeqCst) {
                return true;
            }
        }
    }

    /// Decrement the active count, wakeing up scheduler
    /// if you were the last running task.
    fn active_dec(&self) {
        // This should not effect the flags
        let count = self.active.fetch_sub(1, Ordering::SeqCst);
        if count & REF_COUNT == 1 {
            self.work_done.take().map(|p| p.pulse());
        }
    }
}

/// Task queue back-end.
pub struct Backend {
    inner: Arc<Inner>
}

impl Backend {
    /// Create a new back-end.
    pub fn new() -> Backend {
        Backend {
            inner: Arc::new(Inner{
                active: AtomicUsize::new(0),
                work_done: Atom::empty()
            })
        }
    }

    /// Start a task that will run once all the Handle's have
    /// been completed.
    pub fn start(&self, mut deps: Vec<Handle>, task: Box<FnBox() + Send>) -> Handle {
        let (signal, complete) = Signal::new();

        let pulse = if deps.len() == 0 {
            let (pulse, t) = Signal::new();
            t.pulse();
            pulse
        } else if deps.len() == 1 {
            // If only one, we can just use the handle in it's raw form
            deps.pop().unwrap()
        } else {
            let barrier = Barrier::new(deps);
            barrier.signal()
        };

        let inner = self.inner.clone();
        pulse.callback(move || {
            if inner.try_active_inc() {
                thread::spawn(move || {
                    let inner = inner;
                    task();
                    complete.pulse();
                    inner.active_dec();
                });
            }
        });
        signal
    }

    /// Kill the backend, wait until the condition is satisfied.
    pub fn exit(&self, wait: Wait) {
        let (signal, pulse) = Signal::new();
        // Install the pulse (if needed)
        match wait {
            Wait::Active | Wait::Pending => {
                self.inner.work_done.swap(pulse);
            }
            Wait::None => {
                pulse.pulse()
            }
        }

        // read the current active count, OR in the BLOCK
        // flag if needed for the wait
        let count = match wait {
            Wait::None | Wait::Active => {
                self.inner.active.fetch_or(BLOCK, Ordering::SeqCst)
            }
            Wait::Pending => {
                self.inner.active.load(Ordering::SeqCst)
            }
        };

        // Wait until the count is equal to 0.
        if count & REF_COUNT == 0 {
            return;
        } else {
            signal.wait().unwrap();
        }
    }
}
