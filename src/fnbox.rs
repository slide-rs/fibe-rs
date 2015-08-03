/// A simple version of fnbox found in libstd
pub trait FnBox {
	/// Call the box consomuming the fnbox
    fn call_box(self: Box<Self>);
}

impl<F: FnOnce()> FnBox for F {
    fn call_box(self: Box<Self>) {
        (*self)()
    }
}
