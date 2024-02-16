//! Bindings to the JS API.

use js_sys::WebAssembly::Global;
use js_sys::{Function, Object};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use web_sys::AudioWorkletNodeOptions;

use super::super::super::js::GlobalExt;
use super::Data;

#[wasm_bindgen]
extern "C" {
	/// Returns the constructor of [`TextEncoder`](https://developer.mozilla.org/en-US/docs/Web/API/TextEncoder).
	#[wasm_bindgen(method, getter, js_name = TextEncoder)]
	pub(super) fn text_encoder(this: &GlobalExt) -> JsValue;

	/// Extension for [`BaseAudioContext`](web_sys::BaseAudioContext).
	pub(super) type BaseAudioContextExt;

	/// Returns our custom `registered` property.
	#[wasm_bindgen(method, getter, js_name = __web_thread_registered)]
	pub(super) fn registered(this: &BaseAudioContextExt) -> Option<bool>;

	/// Sets our custom `registered` property.
	#[wasm_bindgen(method, setter, js_name = __web_thread_registered)]
	pub(super) fn set_registered(this: &BaseAudioContextExt, value: bool);

	/// Binding to [`queueMicroTask()`](https://developer.mozilla.org/en-US/docs/Web/API/queueMicrotask).
	#[wasm_bindgen(js_name = queueMicrotask)]
	pub(super) fn queue_microtask(closure: &Function);

	/// Extension for [`AudioWorkletNodeOptions`].
	#[wasm_bindgen(extends = AudioWorkletNodeOptions)]
	pub(super) type AudioWorkletNodeOptionsExt;

	/// Returns [`AudioWorkletNodeOptions.processorOptions`](https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletNode/AudioWorkletNode#processoroptions).
	#[wasm_bindgen(method, getter, js_name = processorOptions)]
	pub(super) fn get_processor_options(
		this: &AudioWorkletNodeOptionsExt,
	) -> Option<ProcessorOptions>;

	/// Type for [`AudioWorkletNodeOptions.processorOptions`](https://developer.mozilla.org/en-US/docs/Web/API/AudioWorkletNode/AudioWorkletNode#processoroptions).
	#[wasm_bindgen(extends = Object)]
	#[derive(Default)]
	pub(super) type ProcessorOptions;

	/// Returns our custom `data` property.
	#[wasm_bindgen(method, getter, js_name = __web_thread_data)]
	pub(super) fn data(this: &ProcessorOptions) -> *const Data;

	/// Sets our custom `data` property.
	#[wasm_bindgen(method, setter, js_name = __web_thread_data)]
	pub(super) fn set_data(this: &ProcessorOptions, value: *const Data);

	/// Type of [`WebAssembly.Module.exports()`s return value](https://developer.mozilla.org/en-US/docs/WebAssembly/JavaScript_interface/Module/exports_static).
	pub(super) type Exports;

	/// [`wasm-bindgen`](wasm_bindgen)s thread destruction function.
	#[wasm_bindgen(method, js_name = __wbindgen_thread_destroy)]
	pub(super) unsafe fn thread_destroy(this: &Exports, tls_base: &Global, stack_alloc: &Global);

	/// Base address of [`wasm-bindgen`](wasm_bindgen)s TLS memory.
	#[wasm_bindgen(method, getter, js_name = __tls_base)]
	pub(super) fn tls_base(this: &Exports) -> Global;

	/// Base address of [`wasm-bindgen`](wasm_bindgen)s thread stack memory.
	#[wasm_bindgen(method, getter, js_name = __stack_alloc)]
	pub(super) fn stack_alloc(this: &Exports) -> Global;

	/// Dictionary type of [`GlobalDescriptor`](https://developer.mozilla.org/en-US/docs/WebAssembly/JavaScript_interface/Global/Global#descriptor).
	#[wasm_bindgen(extends = Object)]
	pub(super) type GlobalDescriptor;

	/// Setter for [`GlobalDescriptor.value`](https://developer.mozilla.org/en-US/docs/WebAssembly/JavaScript_interface/Global/Global#descriptor) property.
	#[wasm_bindgen(method, setter, js_name = value)]
	pub(super) fn set_value(this: &GlobalDescriptor, value: &str);
}
