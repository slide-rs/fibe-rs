
use std::boxed::FnBox;
use pulse::Signal;
use bran;
use {Schedule, TaskBuilder};

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
            WaitState::Ready => { self.extend().start(sched); },
            WaitState::Pending(signal) => { self.extend().after(signal).start(sched); },
            WaitState::Completed => (),
        }
    }
}

impl Task for Box<FnBox(&mut Schedule) + Send + 'static> {
    fn run(self: Box<Self>, sched: &mut Schedule) {
        self.call_box((sched,))
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
    fn start(self, sched: &mut Schedule) {
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

impl ResumableTask for bran::Handle {
    fn resume(&mut self, _: &mut Schedule) -> WaitState {
        match self.run() {
            bran::fiber::State::Finished |
            bran::fiber::State::Panicked => {
                WaitState::Completed
            }
            bran::fiber::State::PendingTimeout(sig, _) |
            bran::fiber::State::Pending(sig) => {
                WaitState::Pending(sig)
            }
        }
    }
}