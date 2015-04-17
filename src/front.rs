//! Front-end module for the task queue. The front-end exists
//! on the user side, allowing to add more tasks to the queue.

use std::sync::Arc;
use std::thread;
use pulse::Pulse;
use {Handle, Wait};
use back::Backend;


/// Queue front-end.
pub struct Frontend {
    backend: Arc<Backend>,
    link: thread::JoinHandle<()>,
}

impl Frontend {
    /// Create a new front-end with an associated
    /// back-end automatically.
    pub fn new() -> Frontend {
        let backend = Arc::new(Backend::new());
        let back = backend.clone();
        let (p, t) = Pulse::new();
        let front = Frontend {
            backend: back,
            link: thread::spawn(move || {
                backend.run(t);
            }),
        };
        p.wait().unwrap();
        front
    }

    /// Add a new task with selected dependencies. This doesn't interrupt any
    /// tasks in-flight. The task will actually start as soon as all dependencies
    /// are finished.
    pub fn add<F: FnOnce() + Send + 'static>(&mut self, task: F, deps: Vec<Handle>)
               -> Handle {
        self.backend.start(deps, Box::new(task))
    }

    /// Stop the queue, using selected wait mode.
    pub fn die(self, wait: Wait) -> bool {
        self.backend.exit(wait);
        self.link.join().is_ok()
    }
}
