use Schedule;

/// A simple version of fnbox found in libstd
pub trait FnBox {
	/// Call the box consomuming the fnbox
    fn call_box(self: Box<Self>, sched: &mut Schedule);
}

impl<F: FnOnce(&mut Schedule)> FnBox for F {
    fn call_box(self: Box<Self>, sched: &mut Schedule) {
        (*self)(sched)
    }
}
