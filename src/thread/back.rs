//! Back-end module for the task queue. The back-end is running
//! on a separate thread. All it does is listening to a command
//! channel and starting new tasks when the time comes.

use std::sync::{Arc, Mutex};
use std::thread;

use pulse::*;

use {Wait, Schedule, FnBox};

/// Task queue back-end.
pub struct Inner {
    shutdown: bool,
    running: usize,
    wake: Option<Pulse>
}

pub struct Backend(Mutex<Inner>);

impl Backend {
    /// Create a new back-end.
    pub fn new() -> Arc<Backend> {
        Arc::new(Backend(Mutex::new(Inner{
            shutdown: false,
            running: 0,
            wake: None
        })))
    }

    /// Start a task that will run once all the Handle's have
    /// been completed.
    pub fn start(back: Arc<Backend>, task: Box<FnBox+Send>, mut after: Vec<Signal>) {
        // Create the wait signal if needed
        let signal = if after.len() == 0 {
            Signal::pulsed()
        } else if after.len() == 1 {
            after.pop().unwrap()
        } else {
            Barrier::new(&after).signal()
        };

        signal.callback(move || {
            let mut g = back.0.lock().unwrap();
            if !g.shutdown {
                let b = back.clone();
                thread::spawn(move || {
                    let mut b = b;
                    task.call_box(&mut b);
                    let mut g = b.0.lock().unwrap();
                    g.running -= 1;
                    if g.running == 0 {
                        g.wake.take().map(|p| p.pulse());
                    }
                });
                g.running += 1;
            }
        });
    }

    /// Kill the backend, wait until the condition is satisfied.
    pub fn exit(&self, wait: Wait) {
        let mut g = self.0.lock().unwrap();

        // read the current active count, OR in the BLOCK
        // flag if needed for the wait
        match wait {
            Wait::None | Wait::Active => {
                g.shutdown = true;
            }
            Wait::Pending => ()
        };

        if g.running != 0 {
            let (p, t) = Signal::new();
            g.wake = Some(t);
            drop(g);
            p.wait().unwrap();
        }
    }
}

impl<'a> Schedule for Arc<Backend>  {
    fn add_task(&mut self, task: Box<FnBox+Send>, after: Vec<Signal>) {
        Backend::start(self.clone(), task, after)
    }
}
