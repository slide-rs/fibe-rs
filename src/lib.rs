#![deny(missing_docs)]

//! A simple task queue with dependency tracking.

#[macro_use]
extern crate log;
extern crate pulse;
extern crate atom;

extern crate rand;
extern crate libc;
extern crate num_cpus;

#[cfg(feature="fiber")]
extern crate deque;

#[cfg(feature="fiber")]
extern crate bran;

extern crate future_pulse;

#[cfg(feature="fiber")]
mod fiber;

#[cfg(feature="thread")]
mod thread;

mod task;
mod fnbox;

#[cfg(feature="fiber")]
pub use fiber::front::Frontend;

#[cfg(feature="thread")]
pub use thread::Frontend;


use pulse::Signal;

pub use fnbox::FnBox;
pub use self::task::{task, TaskBuilder};

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

/// Abstract representation of a the scheduler, allow for new tasks
/// to be created and enqueued.
pub trait Schedule {
    /// Add a new task with selected dependencies. This doesn't interrupt any
    /// tasks in-flight. The task will actually start as soon as all 
    /// dependencies are finished.
    fn add_task(&mut self, task: Box<FnBox+Send>, after: Vec<Signal>);
}
