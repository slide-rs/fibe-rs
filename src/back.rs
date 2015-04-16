//! Back-end module for the task queue. The back-end is running
//! on a separate thread. All it does is listening to a command
//! channel and starting new tasks when the time comes.

use std::boxed::FnBox;
use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;
use {Handle, Request, Wait};


struct Pending {
    task: Box<FnBox() + Send>,
    dependencies: Vec<Handle>,
}

struct Active {
    link: thread::JoinHandle<()>,
}

/// Task queue back-end.
pub struct Backend {
    output: mpsc::Sender<Request>,
    pending: HashMap<Handle, Pending>,
    active: HashMap<Handle, Active>,
}

impl Backend {
    /// Create a new back-end.
    pub fn new(output: mpsc::Sender<Request>) -> Backend {
        Backend {
            output: output,
            pending: HashMap::new(),
            active: HashMap::new(),
        }
    }

    fn is_queued(&self, h: &Handle) -> bool {
        self.pending.contains_key(h) || self.active.contains_key(h)
    }

    fn launch(&mut self, handle: Handle, task: Box<FnBox() + Send>) {
        let output = self.output.clone();
        self.active.insert(handle.clone(), Active {
            link: thread::spawn(move || {
                (task)();
                let _ = output.send(Request::Done(handle));
            }),
        });
    }

    /// Register the addition of a new task.
    fn on_new(&mut self, handle: Handle, deps: Vec<Handle>, task: Box<FnBox() + Send>) {
        debug_assert!(!self.is_queued(&handle));
        match deps.iter().find(|d| self.is_queued(d)) {
            Some(_) => {
                self.pending.insert(handle, Pending {
                    task: task,
                    dependencies: deps,
                });
            },
            None => self.launch(handle, task),
        }
    }

    /// Register the completion of a task.
    fn on_done(&mut self, handle: Handle) {
        // remove from the active list
        if self.active.remove(&handle).is_none() {
            error!("Finished handle was not active: {:?}", handle);
        }
        // gather items to be launched
        let mut temp = Vec::new();
        for (k, v) in self.pending.iter_mut() {
            v.dependencies.retain(|h| *h != handle);
            if v.dependencies.is_empty() {
                temp.push(k.clone());
            }
        }
        // launch new items
        for h in temp.iter() {
            let pending = self.pending.remove(h).unwrap();
            self.launch(h.clone(), pending.task);
        }
    }

    /// Run the main command dispatcher loop.
    pub fn run(&mut self, input: mpsc::Receiver<Request>) {
        let mut break_on_empty = false;
        for request in input.iter() {
            match request {
                Request::New(handle, deps, task) => {
                    self.on_new(handle, deps, task)
                },
                Request::Done(handle) => {
                    self.on_done(handle);
                    if break_on_empty && self.active.is_empty() {
                        debug_assert!(self.pending.is_empty());
                        break
                    }
                },
                Request::Stop(Wait::None) => break,
                Request::Stop(Wait::Active) => {
                    for (_, v) in self.active.drain() {
                        let _ = v.link.join();
                    }
                    break
                },
                Request::Stop(Wait::Pending) => {
                    break_on_empty = true
                },
            }
        }
    }
}
