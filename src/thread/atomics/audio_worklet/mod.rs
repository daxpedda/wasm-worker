//! Audio worklet extension implementations.

mod js;
mod processor;
mod register;

use std::any::{Any, TypeId};
use std::io::Error;

use js_sys::Object;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{AudioWorkletNode, AudioWorkletNodeOptions, BaseAudioContext, DomException};

use self::js::{AudioWorkletNodeOptionsExt, BaseAudioContextExt};
pub(in super::super) use self::processor::register_processor;
pub(in super::super) use self::register::{register_thread, RegisterThreadFuture};
use super::MAIN_THREAD;
use crate::web::audio_worklet::{AudioWorkletNodeError, ExtendAudioWorkletProcessor};

/// Determined if the current thread is the main thread.
pub(in super::super) fn is_main_thread() -> bool {
	*MAIN_THREAD.get_or_init(super::current_id) == super::current_id()
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
	options: Option<AudioWorkletNodeOptions>,
) -> Result<AudioWorkletNode, AudioWorkletNodeError<P>> {
	let data = Box::new(Data {
		type_id: TypeId::of::<P>(),
		data: Box::new(data),
	});

	// If `processor_options` is set already by the user, don't overwrite it!
	let options: AudioWorkletNodeOptionsExt = options.map_or_else(
		|| Object::new().unchecked_into(),
		AudioWorkletNodeOptions::unchecked_into,
	);
	let processor_options = options.get_processor_options();
	let has_processor_options = processor_options.is_some();
	let processor_options = processor_options.unwrap_or_default();
	let data = Box::into_raw(data);
	processor_options.set_data(data);
	let mut options = AudioWorkletNodeOptions::from(options);

	if !has_processor_options {
		options.processor_options(Some(&processor_options));
	}

	match AudioWorkletNode::new_with_options(context, name, &options) {
		Ok(node) => Ok(node),
		Err(error) => Err(AudioWorkletNodeError {
			// SAFETY: We just made this pointer above.
			data: *unsafe { Box::from_raw(data) }
				.data
				.downcast()
				.expect("wrong type encoded"),
			error: error_from_exception(error),
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
	data: Box<dyn Any>,
}

/// Convert a [`JsValue`] to an [`DomException`] and then to an [`Error`].
fn error_from_exception(error: JsValue) -> Error {
	let error: DomException = error.unchecked_into();

	Error::other(format!("{}: {}", error.name(), error.message()))
}
