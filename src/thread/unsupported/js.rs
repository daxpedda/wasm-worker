//! Bindings to the JS API.

use js_sys::Object;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
	#[wasm_bindgen(extends = Object)]
	pub(super) type MemoryDescriptor;

	#[wasm_bindgen(method, setter, js_name = initial)]
	pub(super) fn set_initial(this: &MemoryDescriptor, value: i32);

	#[wasm_bindgen(method, setter, js_name = maximum)]
	pub(super) fn set_maximum(this: &MemoryDescriptor, value: i32);

	#[wasm_bindgen(method, setter, js_name = shared)]
	pub(super) fn set_shared(this: &MemoryDescriptor, value: bool);
}
