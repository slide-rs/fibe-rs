#![feature(core)]
#![deny(missing_docs)]

//! A simple task queue with dependency tracking.

#[macro_use]
extern crate log;
extern crate pulse;

use pulse::Pulse;

mod back;
mod front;

pub use self::front::Frontend;

/// Task handle, used for referencing a task in flight.
pub type Handle = Pulse;

#[derive(PartialEq, Copy, Clone, Debug)]
/// Wait mode for the front-end termination.
pub enum Wait {
    /// Wait for nothing - terminate immediately.
    None,
    /// Wait for active tasks only, drop the pending.
    Active,
    /// Wait for the whole queue to flush.
    Pending,
}
