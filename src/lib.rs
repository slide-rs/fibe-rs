#![feature(core, libc)]
#![deny(missing_docs)]

//! A simple task queue with dependency tracking.

#[macro_use]
extern crate log;
extern crate pulse;
extern crate atom;
extern crate deque;
extern crate rand;
extern crate libc;
extern crate num_cpus;
extern crate bran;

mod back;
mod front;
mod task;
mod worker;
mod fiber;

pub use self::back::TaskBuilder;
pub use self::front::{Frontend, Schedule};
pub use self::task::{WaitState, task, Task, ResumableTask, IntoTask};
pub use self::fiber::fiber;

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

