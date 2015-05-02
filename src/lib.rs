#![feature(core)]
#![deny(missing_docs)]

//! A simple task queue with dependency tracking.

#[macro_use]
extern crate log;
extern crate pulse;
extern crate atom;

use std::boxed::FnBox;

use pulse::Signal;

mod back;
mod front;

pub use self::front::Frontend;

/// Task handle, used for referencing a task in flight.
pub type Handle = Signal;

/// Wait mode for the front-end termination.
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Wait {
    /// Wait for nothing - terminate immediately.
    None,
    /// Wait for active tasks only, drop the pending.
    Active,
    /// Wait for the whole queue to flush.
    Pending,
}

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
    fn resume(&mut self, add: &mut FnMut(Option<Signal>, Box<Task>)) -> WaitState;
}

/// The building block of a task
pub trait Task {
    /// Run the task consuming it
    fn run(self: Box<Self>, add: &mut FnMut(Option<Signal>, Box<Task>));
}

impl<T> Task for T where T: ResumableTask + 'static {
    fn run(mut self: Box<Self>, add: &mut FnMut(Option<Signal>, Box<Task>)) {
        match self.resume(add) {
            WaitState::Ready => add(None, self),
            WaitState::Pending(signal) => add(Some(signal), self),
            WaitState::Completed => (),
        }
    }
}

impl Task for FnBox(&mut FnMut(Option<Signal>, Box<Task>)) + 'static {
    fn run(self: Box<Self>, add: &mut FnMut(Option<Signal>, Box<Task>)) {
        self.call_box((add,))
    }
}