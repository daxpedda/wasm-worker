//! Audio worklet extensions.

use std::fmt::{Debug, Display, Formatter};
use std::future::Future;
use std::panic::RefUnwindSafe;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{any, error, fmt, io};

use js_sys::{Array, Object};
#[cfg(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "audio-worklet"
))]
use web_sys::{AudioWorkletGlobalScope, BaseAudioContext};
use web_sys::{AudioWorkletNode, AudioWorkletNodeOptions, AudioWorkletProcessor};

#[cfg(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "audio-worklet"
))]
use crate::thread::audio_worklet;
use crate::Thread;

#[cfg(not(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "audio-worklet"
)))]
mod audio_worklet {
	pub(super) struct RegisterThreadFuture;
}
#[cfg(not(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "audio-worklet"
)))]
mod web_sys {
	pub(super) struct AudioWorkletNodeOptions;
	pub(super) struct AudioWorkletProcessor;
	pub(super) struct AudioWorkletNode;
}
#[cfg(not(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "audio-worklet"
)))]
mod js_sys {
	pub(super) struct Array;
	pub(super) struct Object;
}

/// Extension for [`BaseAudioContext`].
#[cfg_attr(
	not(all(
		target_family = "wasm",
		target_os = "unknown",
		feature = "audio-worklet"
	)),
	doc = "",
	doc = "[`BaseAudioContext`]: https://docs.rs/web-sys/0.3.68/web_sys/struct.BaseAudioContext.html"
)]
pub trait BaseAudioContextExt {
	/// Registers a thread at this [`BaseAudioContext`].
	///
	/// # Notes
	///
	/// Unfortunately some browsers are not fully spec-compliant and don't fully
	/// shut-down the thread when the [`closed`] [state] is reached. This may
	/// not only cause memory leaks, but any calls into the Wasm module may lead
	/// to undefined behavior. To avoid this make sure to clean up any resources
	/// before [shutting down the audio worklet].
	///
	/// # Errors
	///
	/// - If a thread was already registered at this [`BaseAudioContext`].
	/// - If the [`BaseAudioContext`] is [`closed`].
	/// - If the main thread does not support spawning threads, see
	///   [`has_spawn_support()`](super::has_spawn_support).
	///
	/// [`closed`]: https://developer.mozilla.org/en-US/docs/Web/API/BaseAudioContext/state#closed
	/// [state]: https://developer.mozilla.org/en-US/docs/Web/API/BaseAudioContext/state
	/// [shutting down the audio worklet]: https://developer.mozilla.org/en-US/docs/Web/API/AudioContext/close
	#[cfg_attr(
		not(all(
			target_family = "wasm",
			target_os = "unknown",
			feature = "audio-worklet"
		)),
		doc = "[`BaseAudioContext`]: https://docs.rs/web-sys/0.3.68/web_sys/struct.BaseAudioContext.html"
	)]
	fn register_thread<F>(self, f: F) -> RegisterThreadFuture
	where
		F: 'static + FnOnce() + Send;

	/// Creates a [`AudioWorkletProcessor`]. This will panic in the
	/// constructor of `P` if `name` didn't correspond to `P` registered with
	/// [`AudioWorkletGlobalScopeExt::register_processor_ext()`].
	///
	/// # Errors
	///
	/// - If [`Self::register_thread()`] was not called on this context yet.
	/// - If [`new AudioWorkletNode`] throws.
	///
	/// [`new AudioWorkletNode`]: https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletNode/AudioWorkletNode
	/// [`AudioWorkletProcessor`]: https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletProcessor
	fn audio_worklet_node<P>(
		&self,
		name: &str,
		data: P::Data,
		options: Option<AudioWorkletNodeOptions>,
	) -> Result<AudioWorkletNode, AudioWorkletNodeError<P>>
	where
		P: 'static + ExtendAudioWorkletProcessor;
}

#[cfg(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "audio-worklet"
))]
impl<T> BaseAudioContextExt for T
where
	BaseAudioContext: From<T>,
	T: AsRef<BaseAudioContext>,
{
	fn register_thread<F>(self, #[allow(clippy::min_ident_chars)] f: F) -> RegisterThreadFuture
	where
		F: 'static + FnOnce() + Send,
	{
		RegisterThreadFuture(audio_worklet::register_thread(self.into(), f))
	}

	fn audio_worklet_node<P>(
		&self,
		name: &str,
		data: P::Data,
		options: Option<AudioWorkletNodeOptions>,
	) -> Result<AudioWorkletNode, AudioWorkletNodeError<P>>
	where
		P: 'static + ExtendAudioWorkletProcessor,
	{
		audio_worklet::audio_worklet_node(self.as_ref(), name, data, options)
	}
}

/// Error returned by [`BaseAudioContextExt::audio_worklet_node()`].
pub struct AudioWorkletNodeError<P>
where
	P: ExtendAudioWorkletProcessor,
{
	/// The passed [`ExtendAudioWorkletProcessor::Data`].
	pub data: P::Data,
	/// The error thrown by [`new AudioWorkletNode`].
	///
	/// [`new AudioWorkletNode`]: https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletNode/AudioWorkletNode
	pub error: io::Error,
}

impl<P> Debug for AudioWorkletNodeError<P>
where
	P: ExtendAudioWorkletProcessor,
{
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("AudioWorkletNodeError")
			.field("data", &any::type_name::<P::Data>())
			.field("error", &self.error)
			.finish()
	}
}

impl<P> Display for AudioWorkletNodeError<P>
where
	P: ExtendAudioWorkletProcessor,
{
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		Display::fmt(&self.error, formatter)
	}
}

impl<P> error::Error for AudioWorkletNodeError<P>
where
	P: ExtendAudioWorkletProcessor,
{
	fn source(&self) -> Option<&(dyn error::Error + 'static)> {
		Some(&self.error)
	}
}

/// Waits for the associated thread to register. See
/// [`BaseAudioContextExt::register_thread()`].
#[derive(Debug)]
pub struct RegisterThreadFuture(audio_worklet::RegisterThreadFuture);

impl Future for RegisterThreadFuture {
	type Output = io::Result<Thread>;

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		Pin::new(&mut self.0).poll(cx)
	}
}

impl RefUnwindSafe for RegisterThreadFuture {}

/// Extension for [`AudioWorkletGlobalScope`].
#[cfg_attr(
	not(all(
		target_family = "wasm",
		target_os = "unknown",
		feature = "audio-worklet"
	)),
	doc = "",
	doc = "[`AudioWorkletGlobalScope`]: https://docs.rs/web-sys/0.3.68/web_sys/struct.AudioWorkletGlobalScope.html"
)]
pub trait AudioWorkletGlobalScopeExt {
	/// Creates a class that extends [`AudioWorkletProcessor`] and calls
	/// [`AudioWorkletGlobalScope.registerProcessor()`]. This is a workaround
	/// for [`wasm-bindgen`] currently unable to extend classes, see
	/// [this `wasm-bindgen` issue](https://github.com/rustwasm/wasm-bindgen/issues/210).
	///
	/// # Notes
	///
	/// [`AudioWorkletGlobalScope.registerProcessor()`] does not sync with it's
	/// corresponding [`AudioWorkletNode`] immediately and requires at least one
	/// yield to the event loop cycle in the [`AudioWorkletNode`]s thread for
	/// [`AudioWorkletNode::new()`] to successfully find the requested
	/// [`AudioWorkletProcessor`] by its name. See [`yield_now_async()`].
	///
	/// # Errors
	///
	/// - If the `name` is empty.
	/// - If a processor with this `name` is already registered.
	/// - If this thread was not spawned by [`web-thread`](crate).
	///
	/// [`AudioWorkletGlobalScope.registerProcessor()`]: https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletGlobalScope/registerProcessor
	/// [`AudioWorkletProcessor`]: https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletProcessor
	/// [`yield_now_async()`]: super::yield_now_async
	#[cfg_attr(
		all(
			target_family = "wasm",
			target_os = "unknown",
			feature = "audio-worklet"
		),
		doc = "[`AudioWorkletNode`]: web_sys::AudioWorkletNode",
		doc = "[`AudioWorkletNode::new()`]: web_sys::AudioWorkletNode::new"
	)]
	#[cfg_attr(
		not(all(
			target_family = "wasm",
			target_os = "unknown",
			feature = "audio-worklet"
		)),
		doc = "[`AudioWorkletNode`]: https://docs.rs/web-sys/0.3.68/web_sys/struct.AudioWorkletNode.html",
		doc = "[`AudioWorkletNode::new()`]: https://docs.rs/web-sys/0.3.68/web_sys/struct.AudioWorkletNode.html#method.new"
	)]
	#[cfg_attr(
		all(target_family = "wasm", target_os = "unknown"),
		doc = "[`wasm-bindgen`]: wasm_bindgen"
	)]
	#[cfg_attr(
		not(all(target_family = "wasm", target_os = "unknown")),
		doc = "[`wasm-bindgen`]: https://docs.rs/wasm-bindgen/0.2.91"
	)]
	fn register_processor_ext<P>(&self, name: &str) -> Result<(), io::Error>
	where
		P: 'static + ExtendAudioWorkletProcessor;
}

#[cfg(all(
	target_family = "wasm",
	target_os = "unknown",
	feature = "audio-worklet"
))]
impl AudioWorkletGlobalScopeExt for AudioWorkletGlobalScope {
	fn register_processor_ext<P>(&self, name: &str) -> Result<(), io::Error>
	where
		P: 'static + ExtendAudioWorkletProcessor,
	{
		audio_worklet::register_processor::<P>(name)
	}
}

/// Extends type with [`AudioWorkletProcessor`].
///
/// [`AudioWorkletProcessor`]: https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletProcessor
pub trait ExtendAudioWorkletProcessor {
	/// Data passed into [`Self::new()`] when using
	/// [`BaseAudioContextExt::audio_worklet_node()`].
	type Data: 'static + Send;

	/// Equivalent to constructor.
	fn new(
		this: AudioWorkletProcessor,
		data: Option<Self::Data>,
		options: AudioWorkletNodeOptions,
	) -> Self;

	/// Equivalent to [`AudioWorkletProcessor.process()`].
	///
	/// [`AudioWorkletProcessor.process()`]: https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletProcessor/process
	#[allow(unused_variables)]
	fn process(&mut self, inputs: Array, outputs: Array, parameters: Object) -> bool {
		false
	}

	/// Equivalent to [`AudioWorkletProcessor.parameterDescriptors`].
	///
	/// [`AudioWorkletProcessor.parameterDescriptors`]: https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletProcessor/parameterDescriptors
	#[allow(clippy::must_use_candidate)]
	fn parameter_descriptors() -> Array {
		Array::new()
	}
}
