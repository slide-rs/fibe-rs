
use std::cell::RefCell;
use std::sync::Arc;
use std::thread;
use std::sync::mpsc::Receiver;
use std::thread::sleep_ms;

use pulse::Signal;
use rand::{self, Rng};
use deque::{self, Stolen};
use back::{Backend, ReadyTask};
use bran;

use FnBox;

pub enum Command {
    Add(usize, deque::Stealer<ReadyTask>),
    Exit
}

thread_local!(static WORKER: RefCell<Option<Worker>> = RefCell::new(None));

pub struct Worker {
    index: usize,
    back: Arc<Backend>,
    queue: deque::Worker<ReadyTask>,
    command: Option<Receiver<Command>>
}

impl Worker {
    pub fn new(back: Arc<Backend>) -> Worker {
        let (index, worker, rx) = back.new_deque();

        Worker {
            back: back,
            index: index,
            queue: worker,
            command: Some(rx)
        }
    }

    pub fn start(self) {
        let name = format!("Worker {}", self.index);
        let back = self.back.clone();
        let guard = thread::Builder::new().name(name).spawn(move || {
            WORKER.with(|worker| {
                *worker.borrow_mut() = Some(self);
            });
            work();
        }).unwrap();

        back.register_worker(guard);
    }
}

#[inline(never)]
fn work() {
    WORKER.with(|worker| {
        let cmd = worker.borrow_mut().as_mut().unwrap().command.take().unwrap();

        let mut rand = rand::XorShiftRng::new_unseeded();
        let mut stealers: Vec<(usize, deque::Stealer<ReadyTask>)> = Vec::new();

        let mut i = 0;
        let mut run = true;
        let mut backoff = 0;

        while run {
            // Try to grab form our own queue
            if let Some(task) = worker.borrow().as_ref().unwrap().queue.pop() {
                task.run();
                i = 0;
                backoff = 0;
                continue;
            }

            while run {
                i += 1;
    
                // Try to grab from one of the stealers
                if stealers.len() > 0 {
                    let x: usize = rand.gen();
                    let x = x % stealers.len();
                    if let Stolen::Data(task) = stealers[x].1.steal() {
                        task.run();
                        i = 0;
                        backoff = 0;
                        break;
                    }
                }

                // Try to go to sleep
                if i >= stealers.len() * 2 {
                    while let Ok(msg) = cmd.try_recv() {
                        match msg {
                            Command::Add(key, value) => {
                                stealers.push((key, value));
                            }
                            Command::Exit => {
                                run = false;
                            }
                        }
                        i = 0;
                    }

                    if i != 0 {
                        backoff += 1;
                        sleep_ms(backoff);
                        i = stealers.len();
                    }
                }
            }
        }
    });
}

// Use the task on the TLS queue or the queue in the backend
#[inline]
pub fn start(rt: ReadyTask) -> Result<bool, ReadyTask> {
    WORKER.with(|worker| {
        if let Some(worker) = worker.borrow().as_ref() {
            worker.queue.push(rt);
            Ok(true)
        } else {
            Err(rt)
        }
    })
}

/// used for fibers to give them child task spawning
pub struct FiberSchedule;


impl super::Schedule for FiberSchedule {
    fn add_task(&mut self, task: Box<FnBox+Send>, after: Vec<Signal>) {
        let back = WORKER.with(|worker| {
            worker.borrow()
                  .as_ref()
                  .expect("a fiber was resumed outside of a worker")
                  .back.clone()
        });
        Backend::start(back, task, after)
    }
}

pub fn requeue(task: bran::Handle, after: Signal) {
    let back = WORKER.with(|worker| {
        worker.borrow()
              .as_ref()
              .expect("a fiber was resumed outside of a worker")
              .back.clone()
    });
    Backend::enqueue(back, task, after)
}