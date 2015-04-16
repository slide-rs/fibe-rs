#![feature(core, std_misc)]
#![deny(missing_docs)]

//! A simple task queue with dependency tracking.

#[macro_use]
extern crate log;

use std::boxed::FnBox;

mod back;
mod front;

pub use self::front::Frontend;

/// Task handle, used for referencing a task in flight.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Handle(u32);

/// Wait mode for the front-end termination.
pub enum Wait {
    /// Wait for nothing - terminate immediately.
    None,
    /// Wait for active tasks only, drop the pending.
    Active,
    /// Wait for the whole queue to flush.
    Pending,
}

// Rust insists on this enum needing to be public, becase
// `Backend::new()` uses it, even though the `Backend`
// itself is not publicly exported.
#[doc(hidden)]
#[allow(missing_docs)]
pub enum Request {
    New(Handle, Vec<Handle>, Box<FnBox() + Send>),
    Done(Handle),
    Stop(Wait),
}
