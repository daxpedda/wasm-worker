//! Audio worklet extension redirection.

use std::future::Future;
use std::io::{self, Error, ErrorKind};
use std::pin::Pin;
use std::task::{Context, Poll};

use web_sys::BaseAudioContext;

#[cfg(target_feature = "atomics")]
use super::atomics::audio_worklet;
#[cfg(not(target_feature = "atomics"))]
use super::unsupported::audio_worklet;
use super::Thread;

/// Implementation for
/// [`crate::web::audio_worklet::BaseAudioContextExt::register_thread()`].
pub(crate) fn register_thread<F>(context: BaseAudioContext, task: F) -> RegisterThreadFuture
where
	F: 'static + FnOnce() + Send,
{
	RegisterThreadFuture(if super::has_spawn_support() {
		audio_worklet::register_thread(context, task)
	} else {
		audio_worklet::RegisterThreadFuture::error(Error::new(
			ErrorKind::Unsupported,
			"operation not supported on this platform without the atomics target feature and \
			 cross-origin isolation",
		))
	})
}

/// Implementation for [`crate::web::audio_worklet::RegisterThreadFuture`].
#[derive(Debug)]
pub(crate) struct RegisterThreadFuture(audio_worklet::RegisterThreadFuture);

impl Future for RegisterThreadFuture {
	type Output = io::Result<Thread>;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		Pin::new(&mut self.0).poll(cx)
	}
}
