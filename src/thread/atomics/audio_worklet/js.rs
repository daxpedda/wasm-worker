//! Bindings to the JS API.

use js_sys::{Function, Object};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use web_sys::AudioWorkletNodeOptions;

use super::super::super::js::GlobalExt;
use super::Data;

#[wasm_bindgen]
extern "C" {
	#[wasm_bindgen(method, getter, js_name = TextEncoder)]
	pub(super) fn text_encoder(this: &GlobalExt) -> JsValue;

	pub(super) type BaseAudioContextExt;

	#[wasm_bindgen(method, getter, js_name = __web_thread_registered)]
	pub(super) fn registered(this: &BaseAudioContextExt) -> Option<bool>;

	#[wasm_bindgen(method, setter, js_name = __web_thread_registered)]
	pub(super) fn set_registered(this: &BaseAudioContextExt, value: bool);

	#[wasm_bindgen(js_name = queueMicrotask)]
	pub(super) fn queue_microtask(closure: &Function);

	#[wasm_bindgen(extends = AudioWorkletNodeOptions)]
	pub(super) type AudioWorkletNodeOptionsExt;

	#[wasm_bindgen(method, getter, js_name = processorOptions)]
	pub(super) fn get_processor_options(
		this: &AudioWorkletNodeOptionsExt,
	) -> Option<ProcessorOptions>;

	#[wasm_bindgen(extends = Object)]
	#[derive(Default)]
	pub(super) type ProcessorOptions;

	#[wasm_bindgen(method, getter, js_name = __web_thread_data)]
	pub(super) fn data(this: &ProcessorOptions) -> *const Data;

	#[wasm_bindgen(method, setter, js_name = __web_thread_data)]
	pub(super) fn set_data(this: &ProcessorOptions, value: *const Data);
}
