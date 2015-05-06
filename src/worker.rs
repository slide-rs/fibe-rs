
use std::cell::RefCell;
use std::sync::Arc;
use std::boxed::FnBox;
use std::thread;
use std::sync::mpsc::Receiver;
use libc::funcs::posix88::unistd::usleep;
use rand::{self, Rng};
use deque::{self, Stolen};
use back::{Backend};


pub enum Command {
    Add(usize, deque::Stealer<Box<FnBox()+Send>>),
    //Remove(usize),
    Exit
}

thread_local!(static WORKER: RefCell<Option<Worker>> = RefCell::new(None));

pub struct Worker {
    index: usize,
    queue: deque::Worker<Box<FnBox()+Send>>,
    command: Option<Receiver<Command>>
}

impl Worker {
    pub fn new(back: Arc<Backend>) -> Worker {
        let (index, worker, rx) = back.new_deque();

        Worker {
            index: index,
            queue: worker,
            command: Some(rx)
        }
    }

    pub fn start(self) {
        let name = format!("Worker {}", self.index);
        thread::Builder::new().name(name).spawn(move || {
            WORKER.with(|worker| {
                *worker.borrow_mut() = Some(self);
            });
            work();
        }).unwrap();
    }
}

#[inline(never)]
fn work() {
    WORKER.with(|worker| {
        let cmd = worker.borrow_mut().as_mut().unwrap().command.take().unwrap();
        let mut rand = rand::XorShiftRng::new_unseeded();
        let mut stealers: Vec<(usize, deque::Stealer<Box<FnBox()+Send>>)> = Vec::new();

        let mut backoff = 0;
        let mut index = 0;
        let mut run = true;

        while run {
            // Try to grab form our own queue
            if let Some(task) = worker.borrow().as_ref().unwrap().queue.pop() {
                //println!("Mine {:?}", thread::current());
                task();
                backoff = 0;
                index = 0;
                continue;
            }

            while run {
                index += 1;
    
                // Try to grab from one of the stealers
                if stealers.len() > 0 {
                    let x: usize = rand.gen();
                    let x = x % stealers.len();
                    if let Stolen::Data(task) = stealers[x].1.steal() {
                        //println!("Stolen from[{}] {:?}", x, thread::current());
                        task();
                        backoff = 0;
                        index = 0;
                        break;
                    }
                }

                if index > 256 {
                    let recv = cmd.try_recv()
                       .map(|msg| {
                        match msg {
                            Command::Add(key, value) => stealers.push((key, value)),
                            /*Command::Remove(key) => {
                                let mut idx = None;
                                for (i, &(k, _)) in stealers.iter().enumerate() {
                                    if key == k {
                                        idx = Some(i);
                                    }
                                }
                                idx.map(|i| stealers.swap_remove(i));
                            },*/
                            Command::Exit => {
                                run = false;
                            }
                        }
                    }).ok().is_some();

                    if !recv {
                        backoff += 5;
                        unsafe { usleep(backoff) };
                    }
                }                
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