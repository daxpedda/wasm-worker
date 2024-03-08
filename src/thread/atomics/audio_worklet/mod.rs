//! Audio worklet extension implementations.

mod js;
#[cfg(feature = "message")]
pub(super) mod main;
mod processor;
pub(super) mod register;

use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::sync::OnceLock;

use js_sys::{JsString, Object, Reflect};
use wasm_bindgen::JsCast;
use web_sys::{AudioWorkletNode, AudioWorkletNodeOptions, BaseAudioContext};

use self::js::{AudioWorkletNodeOptionsExt, BaseAudioContextExt};
pub(in super::super) use self::processor::register_processor;
pub(in super::super) use self::register::{
	register_thread, AudioWorkletHandle, RegisterThreadFuture,
};
use super::super::js::GlobalExt;
pub(in super::super) use super::is_main_thread;
use crate::web::audio_worklet::{AudioWorkletNodeError, ExtendAudioWorkletProcessor};

/// Macro to cache conversions from [`String`]s to [`JsString`]s to avoid
/// `TextDecoder` when not available and the overhead of creating [`JsString`].
macro_rules! js_string {
	($($(#[$doc:meta])* static $name:ident = $value:literal;)*) => {
		thread_local! {
			$(
				$(#[$doc])*
				static $name: JsString = if HAS_TEXT_DECODER.with(bool::clone) {
					JsString::from($value)
				} else {
					/// There is currently no nice way in Rust to convert
					/// [`String`]s into `Vec<u32>`s without allocation, so we
					/// cache it.
					static NAME: OnceLock<Vec<u32>> = OnceLock::new();

					JsString::from_code_point(
						NAME.get_or_init(|| $value.chars().map(u32::from).collect())
							.as_slice(),
					)
					.expect("found invalid Unicode")
				};
			)*
		}
	};
}

thread_local! {
	/// Caches if this audio worklet supports [`TextDecoder`]. It is possible
	/// that users will add a polyfill, so we don't want to assume that all
	/// audio worklets have the same support.
	///
	/// [`TextDecoder`]: https://developer.mozilla.org/en-US/docs/Web/API/TextDecoder
	static HAS_TEXT_DECODER: bool = !js_sys::global()
		.unchecked_into::<GlobalExt>()
		.text_decoder()
		.is_undefined();
}

js_string! {
	/// Name of our custom property on [`AudioWorkletNodeOptions`].
	static DATA_PROPERTY_NAME = "__web_thread_data";

	/// Name of the
	/// [`AudioWorkletNodeOptions.processorOptions`](https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletNode/AudioWorkletNode#processoroptions)
	/// property.
	static PROCESSOR_OPTIONS_PROPERTY_NAME = "processorOptions";
}

/// Returns [`true`] if this context has a registered thread.
pub(in super::super) fn is_registered(context: &BaseAudioContext) -> bool {
	matches!(
		context.unchecked_ref::<BaseAudioContextExt>().registered(),
		Some(true)
	)
}

/// Implementation for
/// [`crate::web::audio_worklet::BaseAudioContextExt::audio_worklet_node()`].
pub(in super::super) fn audio_worklet_node<P: 'static + ExtendAudioWorkletProcessor>(
	context: &BaseAudioContext,
	name: &str,
	data: P::Data,
	options: Option<&AudioWorkletNodeOptions>,
) -> Result<AudioWorkletNode, AudioWorkletNodeError<P>> {
	// If `processor_options` is set already by the caller, don't overwrite it!
	let options: Cow<'_, AudioWorkletNodeOptionsExt> = options.map_or_else(
		|| Cow::Owned(Object::new().unchecked_into()),
		|options| Cow::Borrowed(options.unchecked_ref()),
	);
	let processor_options = options.get_processor_options();
	let has_processor_options = processor_options.is_some();

	let data = Box::new(Data {
		type_id: TypeId::of::<P>(),
		value: Box::new(data),
		empty: !has_processor_options,
	});
	let processor_options = processor_options.unwrap_or_default();
	let data = Box::into_raw(data);
	processor_options.set_data(data);

	if !has_processor_options {
		options.set_processor_options(Some(&processor_options));
	}

	let result = AudioWorkletNode::new_with_options(context, name, &options);

	if has_processor_options {
		DATA_PROPERTY_NAME
			.with(|name| Reflect::delete_property(&processor_options, name))
			.expect("expected `processor_options` to be an `Object`");
	} else {
		PROCESSOR_OPTIONS_PROPERTY_NAME
			.with(|name| Reflect::delete_property(&options, name))
			.expect("expected `AudioWorkletNodeOptions` to be an `Object`");
	}

	match result {
		Ok(node) => Ok(node),
		Err(error) => Err(AudioWorkletNodeError {
			// SAFETY: We just made this pointer above and `new AudioWorkletNode` has to guarantee
			// that on error transmission failed to avoid double-free.
			data: *unsafe { Box::from_raw(data) }
				.value
				.downcast()
				.expect("wrong type encoded"),
			error: super::error_from_exception(error),
		}),
	}
}

/// Data stored in [`AudioWorkletNodeOptions.processorOptions`] to transport
/// [`ExtendAudioWorkletProcessor::Data`].
///
/// [`AudioWorkletNodeOptions.processorOptions`]: https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletNode/AudioWorkletNode#processoroptions
struct Data {
	/// [`TypeId`] to compare to the type when arriving at the constructor.
	type_id: TypeId,
	/// [`ExtendAudioWorkletProcessor::Data`].
	value: Box<dyn Any>,
	/// If [`AudioWorkletNodeOptions.processorOptions`] was empty.
	///
	/// [`AudioWorkletNodeOptions.processorOptions`]: https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletNode/AudioWorkletNode#processoroptions
	empty: bool,
}
