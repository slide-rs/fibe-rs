//! Back-end module for the task queue. The back-end is running
//! on a separate thread. All it does is listening to a command
//! channel and starting new tasks when the time comes.

use std::sync::atomic::*;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Sender, Receiver, channel};
use std::collections::HashMap;
use std::thread;

use pulse::*;
use deque;
use num_cpus;

use {Handle, Wait, Task, Schedule, IntoTask};
use worker;

struct Inner {
    index: usize,
    stealers: HashMap<usize, deque::Stealer<ReadyTask>>,
    workers: HashMap<usize, Sender<worker::Command>>,
    joins: Vec<thread::JoinHandle<()>>
}

/// Task queue back-end.
pub struct Backend {
    active: AtomicBool,
    global_queue: Mutex<deque::Worker<ReadyTask>>,
    workers: Mutex<Inner>,
}

pub struct ReadyTask {
    // the task to be run
    task: Box<Task+Send>,
    // this is dropped when a task is complete
    // it may be cloned to `extend` the life of
    // a task
    complete_ack: Option<(Signal, DoneAck)>
}

impl ReadyTask {
    pub fn run(self, back: Arc<Backend>) {
        let ReadyTask{task, mut complete_ack} = self;

        let mut back = (back, &mut complete_ack);
        task.run(&mut back);
    }
}

impl Backend {
    /// Create a new back-end.
    pub fn new() -> Arc<Backend> {
        let buffer = deque::BufferPool::new();
        let (worker, stealer) = buffer.deque();

        let mut map = HashMap::new();
        map.insert(0, stealer);

        let back = Arc::new(Backend {
            active: AtomicBool::new(false),
            global_queue: Mutex::new(worker),
            workers: Mutex::new(Inner {
                index: 1,
                stealers: map,
                workers: HashMap::new(),
                joins: Vec::new()
            }),
        });

        for _ in 0..num_cpus::get() {
            worker::Worker::new(back.clone()).start();
        }
        back
    }

    /// Start a task on the global work queue
    fn start_on_global_queue(&self, rt: ReadyTask) {
        let guard = self.global_queue.lock().unwrap();
        guard.push(rt);
    }

    /// Start a task that will run once all the Handle's have
    /// been completed.
    pub fn start(back: Arc<Backend>, mut task: TaskBuilder,
                 ack: &mut Option<(Signal, DoneAck)>) -> Handle {

        // Create or reuse the DoneAck
        let (done_signal, ack) = if task.extend {
            ack.take().expect("No parent thread to extend, A task may only be extended once.")
        } else {
            let (signal, complete) = Signal::new();
            (signal, DoneAck::new(complete))
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
            if !back.active.load(Ordering::SeqCst) {
                let try_thread = worker::start(ReadyTask {
                    task: task.inner,
                    complete_ack: Some((sig, ack)),
                });

                match try_thread {
                    Ok(b) => b,
                    Err(rt) => {
                        back.start_on_global_queue(rt);
                        true
                    }
                };
            }
        });

        done_signal
    }

    /// Kill the backend, wait until the condition is satisfied.
    pub fn exit(&self, wait: Wait) {
        // read the current active count, OR in the BLOCK
        // flag if needed for the wait
        match wait {
            Wait::None | Wait::Active => {
                self.active.store(true, Ordering::SeqCst);
            }
            Wait::Pending => ()
        };

        let mut guard = self.workers.lock().unwrap();
        for (_, send) in guard.workers.iter() {
            let _ = send.send(worker::Command::Exit);
        }

        while let Some(join) = guard.joins.pop() {
            join.join().unwrap();
        }
    }

    /// Create a new deque
    pub fn new_deque(&self) -> (usize,
                                deque::Worker<ReadyTask>,
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

    ///
    pub fn register_worker(&self, handle: thread::JoinHandle<()>) {
        let mut guard = self.workers.lock().unwrap();
        guard.joins.push(handle);
    }
}

impl<'a> Schedule for (Arc<Backend>, &'a mut Option<(Signal, DoneAck)>)  {
    fn add_task(&mut self, task: TaskBuilder) -> Handle {
        Backend::start(self.0.clone(), task, self.1)
    }
}

impl<'a> Schedule for &'a mut (Arc<Backend>, &'a mut Option<(Signal, DoneAck)>)  {
    fn add_task(&mut self, task: TaskBuilder) -> Handle {
        Backend::start(self.0.clone(), task, self.1)
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