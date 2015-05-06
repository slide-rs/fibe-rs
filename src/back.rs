//! Back-end module for the task queue. The back-end is running
//! on a separate thread. All it does is listening to a command
//! channel and starting new tasks when the time comes.

use std::sync::atomic::*;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Sender, Receiver, channel};
use std::boxed::FnBox;
use std::collections::HashMap;
use atom::*;
use pulse::*;
use deque;
use num_cpus;

use {Handle, Wait, Task, Schedule, IntoTask};
use worker;

// Todo 64bit version
const BLOCK: usize = 0x8000_0000;
const REF_COUNT: usize = 0x7FFF_FFFF;

struct Inner {
    index: usize,
    stealers: HashMap<usize, deque::Stealer<Box<FnBox()+Send>>>,
    workers: HashMap<usize, Sender<worker::Command>>
}

/// Task queue back-end.
pub struct Backend {
    active: AtomicUsize,
    work_done: Atom<Pulse>,

    pub global_queue: Mutex<deque::Worker<Box<FnBox()+Send>>>,
    workers: Mutex<Inner>,
}

impl Backend {
    /// Create a new back-end.
    pub fn new() -> Arc<Backend> {
        let buffer = deque::BufferPool::new();
        let (worker, stealer) = buffer.deque();

        let mut map = HashMap::new();
        map.insert(0, stealer);

        let back = Arc::new(Backend {
            active: AtomicUsize::new(0),
            work_done: Atom::empty(),
            global_queue: Mutex::new(worker),
            workers: Mutex::new(Inner {
                index: 0,
                stealers: map,
                workers: HashMap::new()
            }),
        });

        for _ in 0..num_cpus::get() {
            worker::Worker::new(back.clone()).start();
        }
        back
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
            if value == self.active.compare_and_swap(value, value+1, Ordering::SeqCst) {
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
    pub fn start(back: Arc<Backend>, mut task: TaskBuilder,
                 ack: Option<(Signal, Arc<DoneAck>)>) -> Handle {

        // Create or reuse the DoneAck
        let (done_signal, ack) = if task.extend {
            ack.expect("No parent thread to extend")
        } else {
            let (signal, complete) = Signal::new();
            (signal, Arc::new(DoneAck::new(complete)))
        };

        // Create the wait signal if needed
        let signal = if task.wait.len() == 0 {
            Signal::pulsed()
        } else if task.wait.len() == 1 {
            task.wait.pop().unwrap()
        } else {
            Barrier::new(&task.wait).signal()
        };

        let sig = done_signal.clone();
        signal.callback(move || {
            if back.try_active_inc() {
                worker::start(&back.clone(), Box::new(move || {
                    let mut back = (back, sig, ack);
                    task.inner.run(&mut back);

                    let (back, sig, ack) = back;
                    // Drop this before active_dec so that any pending
                    // tasks are started before we try and signal the backend
                    // that work is done
                    drop((sig, ack));
                    back.active_dec();
                }));
            }
        });

        done_signal
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
        if count & REF_COUNT != 0 {
            signal.wait().unwrap();
        }

        let guard = self.workers.lock().unwrap();
        for (_, send) in guard.workers.iter() {
            let _ = send.send(worker::Command::Exit);
        }
    }

    /// Create a new deque
    pub fn new_deque(&self) -> (usize,
                                deque::Worker<Box<FnBox()+Send>>,
                                Receiver<worker::Command>) {

        let buffer = deque::BufferPool::new();
        let (worker, stealer) = buffer.deque();
        let (send, recv) = channel();
        let mut guard = self.workers.lock().unwrap();
        let index = guard.index;
        guard.index += 1;
        for (&key, stealer) in guard.stealers.iter() {
            send.send(worker::Command::Add(key, stealer.clone())).unwrap();
        }
        for (_, workers) in guard.workers.iter() {
            workers.send(worker::Command::Add(index, stealer.clone())).unwrap();
        }
        guard.stealers.insert(index, stealer);
        guard.workers.insert(index, send);
        (index, worker, recv)
    }
}

impl Schedule for (Arc<Backend>, Signal, Arc<DoneAck>)  {
    fn add_task(&self, task: TaskBuilder) -> Handle {
        Backend::start(self.0.clone(), task, Some((self.1.clone(), self.2.clone())))
    }
}

impl<'a> Schedule for &'a mut (Arc<Backend>, Signal, Arc<DoneAck>)  {
    fn add_task(&self, task: TaskBuilder) -> Handle {
        Backend::start(self.0.clone(), task, Some((self.1.clone(), self.2.clone())))
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

/// A structure to help build a task
pub struct TaskBuilder {
    /// The task to be run
    inner: Box<Task+Send>,
    /// is the task extended or not
    extend: bool,
    /// The signals to wait on
    wait: Vec<Signal>
}

impl TaskBuilder {
    /// Create a new TaskBuilder around `t`
    pub fn new<T>(t: T) -> TaskBuilder where T: IntoTask {
        TaskBuilder {
            inner: t.into_task(),
            extend: false,
            wait: Vec::new()
        }
    }

    /// A task extend will extend the lifetime of the parent task
    /// Externally to this task the Handle will not show as complete
    /// until both the parent, and child are completed.
    ///
    /// A parent should not wait on the child task if it is extended
    /// the parent's lifetime. As this will deadlock.
    pub fn extend(mut self) -> TaskBuilder {
        self.extend = true;
        self
    }

    /// Start the task only after `signal` is asserted
    pub fn after(mut self, signal: Signal) -> TaskBuilder {
        self.wait.push(signal);
        self
    }

    /// Start the task using the supplied scheduler
    pub fn start(self, sched: &mut Schedule) -> Signal {
        sched.add_task(self)
    }
}