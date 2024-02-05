//! Audio worklet extension implementations.

use std::borrow::Cow;
use std::future::Future;
use std::io::{self, Error};
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use web_sys::BaseAudioContext;

use super::super::Thread;

/// Implementation for
/// [`crate::web::audio_worklet::BaseAudioContextExt::register_thread()`].
pub(in super::super) fn register_thread<F>(
	_: Cow<'_, BaseAudioContext>,
	_: F,
) -> RegisterThreadFuture<'_> {
	unreachable!("reached `register_thread()` without atomics target feature")
}

/// Implementation for [`crate::web::audio_worklet::RegisterThreadFuture`].
#[derive(Debug)]
pub(in super::super) struct RegisterThreadFuture<'context> {
	/// The error returned because the atomic target is not enabled.
	error: Option<Error>,
	/// Holds the `context` lifetime.
	context: PhantomData<&'context ()>,
}

impl Future for RegisterThreadFuture<'_> {
	type Output = io::Result<Thread>;

	fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
		Poll::Ready(Err(self.error.take().expect("polled after completion")))
	}
}

impl RegisterThreadFuture<'_> {
	/// Create a [`RegisterThreadFuture`] that returns `error`.

	pub(in super::super) const fn error(error: Error) -> Self {
		Self {
			error: Some(error),
			context: PhantomData,
		}
	}

	/// Remove the lifetime.
	#[allow(clippy::unused_self)]
	pub(in super::super) fn into_static(self) -> RegisterThreadFuture<'static> {
		unreachable!("reached `into_static()` without atomics target feature")
	}
}
