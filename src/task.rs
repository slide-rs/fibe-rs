
use std::boxed::FnBox;
use pulse::{Signal, Barrier, Signals};
use Schedule;

/// Wait mode for a task
#[derive(Clone, Debug)]
pub enum WaitState {
    /// The Task is ready to run - can be scheduled immediately
    Ready,
    /// The Task has completed and can be deleted
    Completed,
    /// The Task is pending on a signal.
    Pending(Signal)
}

/// This is an abstract trait that represents a long running task.
/// This type of task will run once it's signal
pub trait ResumableTask {
    /// Run your task logic, you must return a WaitState
    /// to yield to the scheduler.
    fn resume(&mut self, sched: &mut Schedule) -> WaitState;
}

/// The building block of a task
pub trait Task {
    /// Run the task consuming it
    fn run(self: Box<Self>, sched: &mut Schedule);
}

impl<T> Task for T where T: ResumableTask + Send + 'static {
    fn run(mut self: Box<Self>, sched: &mut Schedule) {
        match self.resume(sched) {
            WaitState::Ready => { sched.add_child_task(self, None); },
            WaitState::Pending(signal) => { sched.add_child_task(self, Some(signal)); },
            WaitState::Completed => (),
        }
    }
}

impl Task for Box<FnBox(&mut Schedule) + Send + 'static> {
    fn run(self: Box<Self>, sched: &mut Schedule) {
        self.call_box((sched,))
    }
}

/// A structure to help build a task
pub struct TaskBuilder {
    inner: Box<Task+Send>,
    extend: bool,
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
        let TaskBuilder{inner, extend, mut wait} = self;

        let signal = if wait.len() == 0 {
            None
        } else if wait.len() == 1 {
            Some(wait.pop().unwrap())
        } else {
            Some(Barrier::new(&wait).signal())
        };

        if extend {
            sched.add_child_task(inner, signal)
        } else {
            sched.add_task(inner, signal)
        }
    }
}

/// The building block of a task
pub trait RunnableTask {
    /// Run the task consuming it
    fn run(self: Box<Self>, sched: &mut Schedule);
}

/// Convert a foo into a task
pub trait IntoTask: Sized {
    /// Convert yourself into a boxed task
    fn into_task(self) -> Box<Task+Send>;

    /// equivalent of `TaskBuilder::new(self).extend(sched)`
    fn extend(self) -> TaskBuilder {
        TaskBuilder::new(self).extend()
    }

    /// equivalent of `TaskBuilder::new(self).after(sched)`
    fn after(self, signal: Signal) -> TaskBuilder {
        TaskBuilder::new(self).after(signal)
    }

    /// equivalent of `TaskBuilder::new(self).start(sched)`
    fn start(self, sched: &mut Schedule) -> Signal {
        TaskBuilder::new(self).start(sched)
    }
}

impl IntoTask for Box<FnBox(&mut Schedule)+Send+'static> {
    fn into_task(self) -> Box<Task+Send> {
        Box::new(self)
    }
}

impl<T> IntoTask for T where T: ResumableTask+Send+'static {
    fn into_task(self) -> Box<Task+Send> {
        Box::new(self)
    }
}

/// This is a helper function to build a Boxed closure that can be run a task
/// Returns a task builder
pub fn task<F>(f: F) -> TaskBuilder where F: FnOnce(&mut Schedule)+Send+'static {
    let f: Box<FnBox(&mut Schedule)+Send+'static> = Box::new(f);
    TaskBuilder::new(f)
}