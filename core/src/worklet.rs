use std::future::Future;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

use wasm_bindgen_futures::JsFuture;
use web_sys::Worklet;

pub trait WorkletExt {
	fn spawn<F1, F2>(f: F1) -> WorkletFuture
	where
		F1: 'static + FnOnce() -> F2 + Send,
		F2: 'static + Future<Output = ()>;
}

pub struct WorkletFuture(JsFuture);

impl WorkletExt for Worklet {
    fn spawn<F1, F2>(f: F1) -> WorkletFuture
	where
		F1: 'static + FnOnce() -> F2 + Send,
		F2: 'static + Future<Output = ()> {
        todo!()
    }
}

impl Future for WorkletFuture {
	type Output = ();

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		ready!(Pin::new(&mut self.0).poll(cx)).unwrap();
		Poll::Ready(())
	}
}
