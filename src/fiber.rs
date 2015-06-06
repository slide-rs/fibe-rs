
use bran;
use worker::FiberSchedule;
use super::Schedule;

/// Create a fiber
pub fn fiber<F>(f: F) -> bran::fiber::Handle
    where F: FnOnce(&mut Schedule) + Send + 'static {

    bran::spawn(|| {
        f(&mut FiberSchedule);
    })
}