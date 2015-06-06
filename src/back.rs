//! Back-end module for the task queue. The back-end is running
//! on a separate thread. All it does is listening to a command
//! channel and starting new tasks when the time comes.

use std::sync::atomic::*;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Sender, Receiver, channel};
use std::collections::HashMap;
use std::thread;

use bran;
use pulse::*;
use deque;
use num_cpus;

use {Wait, Schedule};
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

/// A ready task
pub struct ReadyTask(bran::Handle);

impl ReadyTask {
    pub fn run(self) {
        use bran::fiber::State;
        let ReadyTask(task) = self;
        match task.run() {
            State::Pending(signal) => {
                worker::FiberSchedule.add_task(task, vec![signal])
            }
            State::PendingTimeout(_, _) => {
                panic!("Timeouts are not supported")
            }
            State::Finished | State::Panicked => ()
        }
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
    pub fn start(back: Arc<Backend>, task: bran::Handle, mut after: Vec<Signal>) {
        // Create the wait signal if needed
        let signal = if after.len() == 0 {
            Signal::pulsed()
        } else if after.len() == 1 {
            after.pop().unwrap()
        } else {
            Barrier::new(&after).signal()
        };

        signal.callback(move || {
            if !back.active.load(Ordering::SeqCst) {
                let try_thread = worker::start(ReadyTask(task));
                match try_thread {
                    Ok(b) => b,
                    Err(rt) => {
                        back.start_on_global_queue(rt);
                        true
                    }
                };
            }
        });
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

impl<'a> Schedule for Arc<Backend>  {
    fn add_task(&mut self, task: bran::Handle, after: Vec<Signal>) {
        Backend::start(self.clone(), task, after)
    }
}
