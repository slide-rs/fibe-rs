//! Front-end module for the task queue. The front-end exists
//! on the user side, allowing to add more tasks to the queue.

use std::sync::Arc;
use std::boxed::FnBox;
use {Handle, Wait, Task};
use back::Backend;
use pulse::Signal;


/// Queue front-end.
pub struct Frontend {
    backend: Arc<Backend>,
}

impl Frontend {
    /// Create a new front-end with an associated
    /// back-end automatically.
    pub fn new() -> Frontend {
        let backend = Arc::new(Backend::new());
        let back = backend.clone();
        let front = Frontend {
            backend: back,
        };
        front
    }

    /// Stop the queue, using selected wait mode.
    pub fn die(self, wait: Wait) -> bool {
        self.backend.exit(wait);
        true
    }
}

/// Abstract representation of a the scheduler, allow for new tasks
/// to be created and enqueued.
pub trait Schedule {
    /// Add a new task with selected dependencies. This doesn't interrupt any
    /// tasks in-flight. The task will actually start as soon as all dependencies
    /// are finished.
    fn add_task(&self, t: Box<Task+Send>, signal: Option<Signal>) -> Handle;
}

impl Schedule for Frontend {
    fn add_task(&self, task: Box<Task+Send>, signal: Option<Signal>) -> Handle {
        Backend::start(self.backend.clone(), task, signal)
    }
}

/// This is a utility trait used to allow the Schedule to be object safe
/// but allow for polymorphic closures to be applied to it
pub trait ScheduleClosure {
    /// Add a new closure with selected dependencies. This doesn't interrupt any
    /// tasks in-flight. The task will actually start as soon as all dependencies
    /// are finished.
    fn add<F>(&self, task: F, signal: Option<Signal>) -> Handle
        where F: FnOnce(&mut Schedule) + Send + 'static;
}

impl<T> ScheduleClosure for T where T: Schedule {
    fn add<F>(&self, task: F, signal: Option<Signal>) -> Handle
        where F: FnOnce(&mut Schedule) + Send + 'static {

        let task: Box<FnBox(&mut Schedule)+Send+'static> = Box::new(task);
        self.add_task(Box::new(task), signal)
    }
}

impl<'a> ScheduleClosure for &'a mut Schedule {
    fn add<F>(&self, task: F, signal: Option<Signal>) -> Handle
        where F: FnOnce(&mut Schedule) + Send + 'static {

        let task: Box<FnBox(&mut Schedule)+Send+'static> = Box::new(task);
        self.add_task(Box::new(task), signal)
    } 
}
