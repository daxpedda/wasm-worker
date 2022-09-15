//! Helper utilities to `try catch` functions.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};

/// Wrap a function in a JS `try catch` block.
pub(crate) fn try_<R>(fn_: impl FnOnce() -> R) -> Result<R, String> {
	#[wasm_bindgen]
	extern "C" {
		/// JS `try catch` block.
		fn __wasm_worker_try(fn_: &mut dyn FnMut()) -> JsValue;
	}

	// This workaround is required because of the limitations of having to pass an
	// `FnMut`, `FnOnce` isn't supported by `wasm_bindgen`.
	let mut fn_ = Some(fn_);
	let mut return_ = None;
	let error =
		__wasm_worker_try(&mut || return_ = Some(fn_.take().expect("called more than once")()));

	return_.ok_or_else(|| {
		error
			.dyn_ref::<js_sys::Error>()
			.map_or_else(|| format!("{error:?}"), |error| error.message().into())
	})
}

pin_project! {
	/// Wrapping a [`Future`] in a JS `try catch` block.
	pub(crate) struct TryFuture<F: Future>{
		#[pin]
		fn_: F
	}
}

impl<F> Future for TryFuture<F>
where
	F: Future,
{
	type Output = Result<F::Output, String>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		match try_(|| self.project().fn_.poll(cx)) {
			Ok(Poll::Ready(return_)) => Poll::Ready(Ok(return_)),
			Ok(Poll::Pending) => Poll::Pending,
			Err(err) => Poll::Ready(Err(err)),
		}
	}
}

impl<F: Future> TryFuture<F> {
	/// Creates a new [`TryFuture`].
	pub(crate) const fn new(fn_: F) -> Self {
		Self { fn_ }
	}
}
