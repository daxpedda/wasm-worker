//! [`AudioWorkletProcessor`] related implementation.
//!
//! [`AudioWorkletProcessor`]: https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletProcessor

use std::any::TypeId;
use std::io::Error;
use std::marker::PhantomData;

use js_sys::{Array, Iterator, JsString, Object, Reflect};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsCast;
use web_sys::{AudioWorkletNodeOptions, DomException};

use super::super::super::js::GlobalExt;
use super::js::{AudioWorkletNodeOptionsExt, ProcessorOptions};
use super::Data;
use crate::web::audio_worklet::ExtendAudioWorkletProcessor;

thread_local! {
	/// Caches if this audio worklet supports [`TextEncoder`]. It is highly
	/// likely that users will add a polyfill, so we don't want to assume that
	/// all audio worklets have the same support.
	///
	/// [`TextEncoder`]: https://developer.mozilla.org/en-US/docs/Web/API/TextEncoder
	static HAS_TEXT_ENCODER: bool = !js_sys::global()
		.unchecked_into::<GlobalExt>()
		.text_encoder()
		.is_undefined();
}

/// Implementation for
/// [`crate::web::audio_worklet::AudioWorkletGlobalScopeExt::register_processor_ext()`].
pub(in super::super::super) fn register_processor<P: 'static + ExtendAudioWorkletProcessor>(
	name: &str,
) -> Result<(), Error> {
	let name = if HAS_TEXT_ENCODER.with(bool::clone) {
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
		mut options: AudioWorkletNodeOptions,
	) -> __WebThreadProcessor {
		let mut processor_data = None;

		if let Some(processor_options) = options
			.unchecked_ref::<AudioWorkletNodeOptionsExt>()
			.get_processor_options()
		{
			let processor_options: ProcessorOptions = processor_options.unchecked_into();

			let data = processor_options.data();

			if !data.is_null() {
				// SAFETY: We only store `*const Data` at `__web_thread_data`.
				let data = unsafe { Box::<Data>::from_raw(data.cast_mut().cast()) };

				if data.type_id == TypeId::of::<P>() {
					processor_data =
						Some(*data.data.downcast::<P::Data>().expect("wrong type encoded"));

					if Object::keys(&processor_options).length() == 1 {
						options.processor_options(None);
					} else {
						thread_local! {
							static DATA_PROPERTY_NAME: JsString =
								if HAS_TEXT_ENCODER.with(bool::clone) {
									JsString::from("__web_thread_data")
								} else {
									JsString::from_code_point(
										"__web_thread_data"
											.chars()
											.map(u32::from)
											.collect::<Vec<_>>()
											.as_slice(),
									)
									.expect("found invalid Unicode")
								};
						}

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