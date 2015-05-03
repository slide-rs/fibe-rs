#![feature(core)]
#![deny(missing_docs)]

//! A simple task queue with dependency tracking.

#[macro_use]
extern crate log;
extern crate pulse;
extern crate atom;

use pulse::Signal;

mod back;
mod front;
mod task;

pub use self::front::{Frontend, Schedule};
pub use self::task::{WaitState, Task, ResumableTask, TaskBuilder};

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

