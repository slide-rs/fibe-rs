//! Front-end module for the task queue. The front-end exists
//! on the user side, allowing to add more tasks to the queue.

use std::sync::mpsc;
use std::thread;
use {Handle, Request, Wait};
use back::Backend;


/// Queue front-end.
pub struct Frontend {
    next_handle: u32,
    output: mpsc::Sender<Request>,
    link: thread::JoinHandle<()>,
}

impl Frontend {
    /// Create a new front-end with an associated
    /// back-end automatically.
    pub fn new() -> Frontend {
        let (sender, receiver) = mpsc::channel();
        Frontend {
            next_handle: 0,
            output: sender.clone(),
            link: thread::spawn(move ||
                Backend::new(sender).run(receiver)
            ),
        }
    }

    /// Add a new task with selected dependencies. This doesn't interrupt any
    /// tasks in-flight. The task will actually start as soon as all dependencies
    /// are finished.
    pub fn add<F: FnOnce() + Send + 'static>(&mut self, task: F, deps: Vec<Handle>)
               -> Result<Handle, ()> {
        let h = Handle(self.next_handle);
        self.next_handle += 1;
        match self.output.send(Request::New(h.clone(), deps, Box::new(task))) {
            Ok(()) => Ok(h),
            Err(_) => Err(()),
        }
    }

    /// Stop the queue, using selected wait mode.
    pub fn die(self, wait: Wait) -> bool {
        self.output.send(Request::Stop(wait)).is_ok() &&
            self.link.join().is_ok()
    }
}
