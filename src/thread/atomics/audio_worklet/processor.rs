//! [`AudioWorkletProcessor`] related implementation.
//!
//! [`AudioWorkletProcessor`]: https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletProcessor

use std::any::TypeId;
use std::io::Error;
use std::marker::PhantomData;
use std::sync::OnceLock;

use js_sys::{Array, Iterator, JsString, Object, Reflect};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsCast;
use web_sys::{AudioWorkletNodeOptions, DomException};

use super::super::super::js::GlobalExt;
use super::js::AudioWorkletNodeOptionsExt;
use super::Data;
use crate::web::audio_worklet::ExtendAudioWorkletProcessor;

/// Macro to cache conversions from [`String`]s to [`JsString`]s to avoid
/// `TextDecoder` and the overhead.
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

/// Implementation for
/// [`crate::web::audio_worklet::AudioWorkletGlobalScopeExt::register_processor_ext()`].
pub(in super::super::super) fn register_processor<P: 'static + ExtendAudioWorkletProcessor>(
	name: &str,
) -> Result<(), Error> {
	let name = if HAS_TEXT_DECODER.with(bool::clone) {
		JsString::from(name)
	} else {
		JsString::from_code_point(name.chars().map(u32::from).collect::<Vec<_>>().as_slice())
			.expect("found invalid Unicode")
	};

	__web_thread_register_processor(
		name,
		__WebThreadProcessorConstructor(Box::new(ProcessorConstructorWrapper::<P>(PhantomData))),
	)
	.map_err(|error| super::error_from_exception(error.into()))
}

/// Holds the user-supplied [`ExtendAudioWorkletProcessor`] while type-erasing
/// it.
#[wasm_bindgen]
struct __WebThreadProcessorConstructor(Box<dyn ProcessorConstructor>);

#[wasm_bindgen]
impl __WebThreadProcessorConstructor {
	/// Calls the underlying [`ExtendAudioWorkletProcessor::new`].
	#[wasm_bindgen]
	#[allow(unreachable_pub)]
	pub fn instantiate(
		&mut self,
		this: web_sys::AudioWorkletProcessor,
		options: AudioWorkletNodeOptions,
	) -> __WebThreadProcessor {
		self.0.instantiate(this, options)
	}

	/// Calls the underlying
	/// [`ExtendAudioWorkletProcessor::parameter_descriptors`].
	#[wasm_bindgen(js_name = parameterDescriptors)]
	#[allow(unreachable_pub)]
	pub fn parameter_descriptors(&self) -> Iterator {
		self.0.parameter_descriptors()
	}
}

/// Wrapper for the user-supplied [`ExtendAudioWorkletProcessor`].
struct ProcessorConstructorWrapper<P: 'static + ExtendAudioWorkletProcessor>(PhantomData<P>);

/// Object-safe version of [`ExtendAudioWorkletProcessor`].
trait ProcessorConstructor {
	/// Calls the underlying [`ExtendAudioWorkletProcessor::new`].
	fn instantiate(
		&mut self,
		this: web_sys::AudioWorkletProcessor,
		options: AudioWorkletNodeOptions,
	) -> __WebThreadProcessor;

	/// Calls the underlying
	/// [`ExtendAudioWorkletProcessor::parameter_descriptors`].
	fn parameter_descriptors(&self) -> Iterator;
}

impl<P: 'static + ExtendAudioWorkletProcessor> ProcessorConstructor
	for ProcessorConstructorWrapper<P>
{
	fn instantiate(
		&mut self,
		this: web_sys::AudioWorkletProcessor,
		options: AudioWorkletNodeOptions,
	) -> __WebThreadProcessor {
		let mut processor_data = None;

		if let Some(processor_options) = options
			.unchecked_ref::<AudioWorkletNodeOptionsExt>()
			.get_processor_options()
		{
			let data = processor_options.data();

			if !data.is_null() {
				// SAFETY: We only store `*const Data` at `__web_thread_data`.
				let data = unsafe { Box::<Data>::from_raw(data.cast_mut().cast()) };

				if data.type_id == TypeId::of::<P>() {
					processor_data =
						Some(*data.data.downcast::<P::Data>().expect("wrong type encoded"));

					// If our custom `data` property was the only things transported, delete
					// `AudioWorkletNodeOptions.processorOptions` entirely.
					if Object::keys(&processor_options).length() == 1 {
						PROCESSOR_OPTIONS_PROPERTY_NAME
							.with(|name| Reflect::delete_property(&processor_options, name))
							.expect("expected `processor_options` to be an `Object`");
					}
					// Otherwise remove our `data` property so its not observable by the user.
					else {
						DATA_PROPERTY_NAME
							.with(|name| Reflect::delete_property(&processor_options, name))
							.expect("expected `processor_options` to be an `Object`");
					}
				}
			}
		}

		__WebThreadProcessor(Box::new(P::new(this, processor_data, options)))
	}

	fn parameter_descriptors(&self) -> Iterator {
		P::parameter_descriptors()
	}
}

/// Holds the user-supplied [`ExtendAudioWorkletProcessor`] while type-erasing
/// it.
#[wasm_bindgen]
struct __WebThreadProcessor(Box<dyn Processor>);

/// Object-safe version of [`ExtendAudioWorkletProcessor`].
trait Processor {
	/// Calls the underlying [`ExtendAudioWorkletProcessor::process`].
	fn process(&mut self, inputs: Array, outputs: Array, parameters: Object) -> bool;
}

impl<P: ExtendAudioWorkletProcessor> Processor for P {
	fn process(&mut self, inputs: Array, outputs: Array, parameters: Object) -> bool {
		ExtendAudioWorkletProcessor::process(self, inputs, outputs, parameters)
	}
}

#[wasm_bindgen]
impl __WebThreadProcessor {
	/// Calls the underlying [`ExtendAudioWorkletProcessor::new`].
	#[wasm_bindgen]
	#[allow(unreachable_pub)]
	pub fn process(&mut self, inputs: Array, outputs: Array, parameters: Object) -> bool {
		self.0.process(inputs, outputs, parameters)
	}
}

/// Entry function for the worklet.
#[wasm_bindgen]
#[allow(unreachable_pub)]
extern "C" {
	#[wasm_bindgen(catch)]
	fn __web_thread_register_processor(
		name: JsString,
		processor: __WebThreadProcessorConstructor,
	) -> Result<(), DomException>;
}
