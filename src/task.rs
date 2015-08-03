
use pulse::Signal;
use future_pulse::Future;
use {Schedule, FnBox};

/// A structure to help build a task
pub struct TaskBuilder<T> {
    /// The task to be run
    task: Box<FnBox+Send>,

    /// The signals to wait on
    wait: Vec<Signal>,

    /// The results
    result: Future<T>
}

impl<T> TaskBuilder<T> {
    /// Start the task only after `signal` is asserted
    pub fn after(mut self, signal: Signal) -> TaskBuilder<T> {
        self.wait.push(signal);
        self
    }

    /// Start the task using the supplied scheduler
    pub fn start(self, sched: &mut Schedule) -> Future<T> {
        let TaskBuilder{task, wait, result} = self;
        sched.add_task(task, wait);
        result
    }
}

/// Create a fiber
pub fn task<F, T:Send+'static>(f: F) -> TaskBuilder<T>
    where F: FnOnce(&mut Schedule) -> T + Send + 'static {

    let (future, set) = Future::new();
    TaskBuilder {
        task: Box::new(move |sched: &mut Schedule| {
            set.set(f(sched));
        }),
        wait: Vec::new(),
        result: future
    }
}