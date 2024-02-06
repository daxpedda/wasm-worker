//! Bindings to the JS API.

use js_sys::Function;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
	pub(super) type BaseAudioContextExt;

	#[wasm_bindgen(method, getter, js_name = __web_thread_registered)]
	pub(super) fn registered(this: &BaseAudioContextExt) -> Option<bool>;

	#[wasm_bindgen(method, setter, js_name = __web_thread_registered)]
	pub(super) fn set_registered(this: &BaseAudioContextExt, value: bool);

	#[wasm_bindgen(js_name = queueMicrotask)]
	pub(super) fn queue_microtask(closure: &Function);
}
