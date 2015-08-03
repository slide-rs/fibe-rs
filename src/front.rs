//! Front-end module for the task queue. The front-end exists
//! on the user side, allowing to add more tasks to the queue.

use std::sync::Arc;
use pulse::Signal;

use back::Backend;
use {Wait, Schedule, FnBox};

/// Queue front-end.
pub struct Frontend {
    backend: Arc<Backend>,
}

impl Frontend {
    /// Create a new front-end with an associated
    /// back-end automatically.
    pub fn new() -> Frontend {
        let backend = Backend::new();
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

impl Drop for Frontend {
    fn drop(&mut self) {
        self.backend.exit(Wait::None)
    }
}

impl Schedule for Frontend {
    fn add_task(&mut self, task: Box<FnBox+Send>, after: Vec<Signal>) {
        Backend::start(self.backend.clone(), task, after)
    }
}
