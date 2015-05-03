//! Back-end module for the task queue. The back-end is running
//! on a separate thread. All it does is listening to a command
//! channel and starting new tasks when the time comes.

use std::thread;
use std::sync::atomic::*;
use std::sync::Arc;
use atom::*;
use pulse::*;

use {Handle, Wait, Task, Schedule};

// Todo 64bit verison
const BLOCK: usize = 0x8000_0000;
const REF_COUNT: usize = 0x7FFF_FFFF;

/// Task queue back-end.
pub struct Backend {
    active: AtomicUsize,
    work_done: Atom<Pulse>
}

impl Backend {
    /// Create a new back-end.
    pub fn new() -> Backend {
        Backend {
            active: AtomicUsize::new(0),
            work_done: Atom::empty()
        }
    }

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

    /// Start a task that will run once all the Handle's have
    /// been completed.
    pub fn start(back: Arc<Backend>, task: Box<Task+Send>, wait: Option<Signal>) -> Handle {
        let (signal, complete) = Signal::new();
        let pulse = wait.unwrap_or_else(|| Signal::pulsed());

        let ack = DoneAck::new(complete);
        pulse.callback(move || {
            if back.try_active_inc() {
                thread::spawn(move || {
                    let mut back = (back, Some(ack));
                    task.run((&mut back) as &mut Schedule);
                    back.0.active_dec();
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
                self.work_done.swap(pulse);
            }
            Wait::None => {
                pulse.pulse()
            }
        }

        // read the current active count, OR in the BLOCK
        // flag if needed for the wait
        let count = match wait {
            Wait::None | Wait::Active => {
                self.active.fetch_or(BLOCK, Ordering::SeqCst)
            }
            Wait::Pending => {
                self.active.load(Ordering::SeqCst)
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

impl Schedule for (Arc<Backend>, Option<DoneAck>)  {
    fn add_task(&self, t: Box<Task+Send>, signal: Option<Signal>) -> Handle {
        Backend::start(self.0.clone(), t, signal)
    }
}

impl<'a> Schedule for &'a mut (Arc<Backend>, Option<DoneAck>)  {
    fn add_task(&self, t: Box<Task+Send>, signal: Option<Signal>) -> Handle {
        Backend::start(self.0.clone(), t, signal)
    }
}

/// This is a shareable object to allow multiple
/// tasks to 
pub struct DoneAck(Option<Pulse>);

impl DoneAck {
    fn new(pulse: Pulse) -> DoneAck {
        DoneAck(Some(pulse))
    }
}

impl Drop for DoneAck {
    fn drop(&mut self) {
        self.0.take().map(|x| x.pulse());
    }
}