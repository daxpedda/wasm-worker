//! Audio worklet extension implementations.

use std::future::Future;
use std::io::{self, Error};
use std::pin::Pin;
use std::task::{Context, Poll};

use web_sys::BaseAudioContext;

use super::super::Thread;

/// Implementation for
/// [`crate::web::audio_worklet::BaseAudioContextExt::register_thread()`].
pub(in super::super) fn register_thread<F>(_: BaseAudioContext, _: F) -> RegisterThreadFuture {
	unreachable!("reached `register_thread()` without atomics target feature")
}

/// Implementation for [`crate::web::audio_worklet::RegisterThreadFuture`].
#[derive(Debug)]
pub(in super::super) struct RegisterThreadFuture(Option<Error>);

impl Future for RegisterThreadFuture {
	type Output = io::Result<Thread>;

	fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
		Poll::Ready(Err(self.0.take().expect("polled after completion")))
	}
}

impl RegisterThreadFuture {
	/// Create a [`RegisterThreadFuture`] that returns `error`.

	pub(in super::super) const fn error(error: Error) -> Self {
		Self(Some(error))
	}
}
