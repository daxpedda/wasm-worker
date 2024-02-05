//! Audio worklet extensions.

use std::borrow::Cow;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

#[cfg(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "audio-worklet"
))]
use web_sys::BaseAudioContext;

#[cfg(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "audio-worklet"
))]
use crate::thread::audio_worklet;
use crate::Thread;

#[cfg(any(
	not(all(target_family = "wasm", target_os = "unknown")),
	not(feature = "audio-worklet")
))]
mod audio_worklet {
	pub(super) struct RegisterThreadFuture<'context>(&'context ());
}

/// Extension for [`BaseAudioContext`].
#[cfg_attr(
	any(
		not(all(target_family = "wasm", target_os = "unknown")),
		not(feature = "audio-worklet")
	),
	doc = "",
	doc = "[`BaseAudioContext`]: https://docs.rs/web-sys/0.3.67/web_sys/struct.BaseAudioContext.html"
)]
pub trait BaseAudioContextExt<'context> {
	/// Registers a thread at this [`BaseAudioContext`].
	///
	/// # Notes
	///
	/// This will automatically clean up thread-local resources when
	/// [`BaseAudioContext`] reaches the [`closed`] [state]. Unfortunately some
	/// browsers are not fully spec-compliant and don't fully shut-down the
	/// thread when the [`closed`] [state] is reached. If any calls into the
	/// Wasm module are made at that point, it could lead to undefined behavior.
	/// To avoid this make sure to clean up any resources before [shutting down
	/// the audio worklet].
	///
	/// # Errors
	///
	/// - If a thread was already registered at this [`BaseAudioContext`].
	/// - If the [`BaseAudioContext`] was closed.
	/// - If the main thread does not support spawning threads, see
	///   [`has_spawn_support()`](super::has_spawn_support).
	///
	/// [`closed`]: https://developer.mozilla.org/en-US/docs/Web/API/BaseAudioContext/state#closed
	/// [state]: https://developer.mozilla.org/en-US/docs/Web/API/BaseAudioContext/state
	/// [shutting down the audio worklet]: https://developer.mozilla.org/en-US/docs/Web/API/AudioContext/close
	#[cfg_attr(
		any(
			not(all(target_family = "wasm", target_os = "unknown")),
			not(feature = "audio-worklet")
		),
		doc = "[`BaseAudioContext`]: https://docs.rs/web-sys/0.3.67/web_sys/struct.BaseAudioContext.html"
	)]
	fn register_thread<F>(self, f: F) -> RegisterThreadFuture<'context>
	where
		F: 'static + FnOnce() + Send;
}

#[cfg(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "audio-worklet"
))]
impl BaseAudioContextExt<'static> for BaseAudioContext {
	fn register_thread<F>(
		self,
		#[allow(clippy::min_ident_chars)] f: F,
	) -> RegisterThreadFuture<'static>
	where
		F: 'static + FnOnce() + Send,
	{
		RegisterThreadFuture(audio_worklet::register_thread(Cow::Owned(self), f))
	}
}

#[cfg(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "audio-worklet"
))]
impl<'context> BaseAudioContextExt<'context> for &'context BaseAudioContext {
	fn register_thread<F>(
		self,
		#[allow(clippy::min_ident_chars)] f: F,
	) -> RegisterThreadFuture<'context>
	where
		F: 'static + FnOnce() + Send,
	{
		RegisterThreadFuture(audio_worklet::register_thread(Cow::Borrowed(self), f))
	}
}

/// Waits for the associated thread to register. See
/// [`BaseAudioContextExt::register_thread()`].
#[derive(Debug)]
pub struct RegisterThreadFuture<'context>(audio_worklet::RegisterThreadFuture<'context>);

impl Future for RegisterThreadFuture<'_> {
	type Output = io::Result<Thread>;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		Pin::new(&mut self.0).poll(cx)
	}
}

impl RegisterThreadFuture<'_> {
	/// Removes the lifetime to [`BaseAudioContext`].
	#[cfg_attr(
		any(
			not(all(target_family = "wasm", target_os = "unknown")),
			not(feature = "audio-worklet")
		),
		doc = "",
		doc = "[`BaseAudioContext`]: https://docs.rs/web-sys/0.3.67/web_sys/struct.BaseAudioContext.html"
	)]
	#[must_use]
	pub fn into_static(self) -> RegisterThreadFuture<'static> {
		RegisterThreadFuture(self.0.into_static())
	}
}
