
use std::cell::RefCell;
use std::sync::Arc;
use std::boxed::FnBox;
use std::thread;
use std::collections::HashMap;
use rand::{self, Rng};
use deque::{self, Stolen};
use back::{TaskBuilder, Backend};

thread_local!(static WORKER: RefCell<Option<Worker>> = RefCell::new(None));

pub struct Worker {
    index: usize,
    back: Arc<Backend>,
    queue: deque::Worker<Box<FnBox()+Send>>,
}

impl Worker {
    pub fn new(back: Arc<Backend>) -> Worker {
        let (index, worker) = back.new_deque();

        Worker {
            index: index,
            back: back,
            queue: worker,
        }
    }

    pub fn start(self) {
        thread::spawn(move || {
            WORKER.with(|worker| {
                *worker.borrow_mut() = Some(self);
            });
            work();
        });
    }
}

fn work() {
    let mut rand = rand::thread_rng();;
    WORKER.with(|worker| {
        let mut stealers = worker.borrow().as_ref().unwrap().back.stealers();
        loop {
            if let Some(task) = worker.borrow().as_ref().unwrap().queue.pop() {
                task();
                continue;
            }

            for i in 0..256 {
                let x: usize = rand.gen();
                let x = x % stealers.len();
                if let Stolen::Data(task) = stealers[x].steal() {
                    task();
                    break;
                } else {
                    if i > 256 - 10 {
                        thread::sleep_ms(1);
                    }
                }
            }

            {
                let mut worker = worker.borrow_mut();
                let worker = worker.as_mut().unwrap();
                stealers = worker.back.stealers();
            }

        }
    });
}

// Use the task on the TLS queue or the queue in the backend
pub fn start(back: &Backend, f: Box<FnBox()+Send>) {
    WORKER.with(|worker| {
        if let Some(worker) = worker.borrow().as_ref() {
            worker.queue.push(f);
        } else {
            let guard = back.global_queue.lock().unwrap();
            guard.push(f);
        }
    });
}