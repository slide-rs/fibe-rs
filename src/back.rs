//! Back-end module for the task queue. The back-end is running
//! on a separate thread. All it does is listening to a command
//! channel and starting new tasks when the time comes.

use std::thread;
use std::sync::atomic::*;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{SyncSender, Sender, Receiver, sync_channel, channel};
use std::boxed::FnBox;
use atom::*;
use pulse::*;

use {Handle, Wait, Task, Schedule, IntoTask};

// Todo 64bit verison
const BLOCK: usize = 0x8000_0000;
const REF_COUNT: usize = 0x7FFF_FFFF;

// Todo, user define...
const MAX_IDLE: usize = 32;

/// Task queue back-end.
pub struct Backend {
    active: AtomicUsize,
    work_done: Atom<Pulse>,

    // Idle queue of threads
    threads: Mutex<Receiver<Sender<Box<FnBox()+Send>>>>,
    queue: Mutex<SyncSender<Sender<Box<FnBox()+Send>>>>
}

impl Backend {
    /// Create a new back-end.
    pub fn new() -> Backend {
        let (tx, rx) = sync_channel(MAX_IDLE);

        Backend {
            active: AtomicUsize::new(0),
            work_done: Atom::empty(),
            threads: Mutex::new(rx),
            queue: Mutex::new(tx)
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

    /// Creates a thread iff needed
    fn thread(&self) -> Sender<Box<FnBox()+Send>> {
        let guard = self.threads.lock().unwrap();
        if let Ok(thread) = guard.try_recv() {
            return thread;
        }

        drop(guard);
        let idle_msg = self.queue.lock().unwrap().clone();

        let (tx, rx): (Sender<Box<FnBox()+Send>>, Receiver<Box<FnBox()+Send>>) = channel();
        thread::spawn(move || {
            // run the first task
            if let Ok(work) = rx.recv() {
                work();
            } else {
                return;
            }

            // run any new messages
            loop {
                let (tx, rx) = channel();
                if let Err(_) = idle_msg.try_send(tx) {
                    return;
                }

                if let Ok(work) = rx.recv() {
                    work();
                } else {
                    return;
                }
            }
        });
        tx
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
                let thread = back.thread();
                thread.send(Box::new(move || {
                    let mut back = (back, sig, ack);
                    task.inner.run(&mut back);

                    let (back, sig, ack) = back;
                    // Drop this before active_dec so that any pending
                    // tasks are started before we try and signal the backend
                    // that work is done
                    drop((sig, ack));
                    back.active_dec();
                })).unwrap();
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
        if count & REF_COUNT == 0 {
            return;
        } else {
            signal.wait().unwrap();
        }
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