//! Bindings to the JS API.

use js_sys::WebAssembly::Global;
use js_sys::{Int32Array, Object, Promise};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen]
extern "C" {
	pub(super) type Meta;

	#[wasm_bindgen(js_namespace = import, js_name = meta)]
	pub(super) static META: Meta;

	#[wasm_bindgen(method, getter)]
	pub(super) fn url(this: &Meta) -> String;

	pub(super) type Atomics;

	#[wasm_bindgen(static_method_of = Atomics, js_name = waitAsync)]
	pub(super) fn wait_async(buf: &Int32Array, index: u32, value: i32) -> WaitAsyncResult;

	#[wasm_bindgen(static_method_of = Atomics, js_name = waitAsync, getter)]
	pub(super) fn has_wait_async() -> JsValue;

	pub(super) type WaitAsyncResult;

	#[wasm_bindgen(method, getter, js_name = async)]
	pub(super) fn async_(this: &WaitAsyncResult) -> bool;

	#[wasm_bindgen(method, getter)]
	pub(super) fn value(this: &WaitAsyncResult) -> Promise;

	pub(super) type Exports;

	#[wasm_bindgen(method, js_name = __wbindgen_thread_destroy)]
	pub(super) unsafe fn thread_destroy(this: &Exports, tls_base: &Global, stack_alloc: &Global);

	#[wasm_bindgen(method, getter, js_name = __tls_base)]
	pub(super) fn tls_base(this: &Exports) -> Global;

	#[wasm_bindgen(method, getter, js_name = __stack_alloc)]
	pub(super) fn stack_alloc(this: &Exports) -> Global;

	#[wasm_bindgen(extends = Object)]
	pub(super) type GlobalDescriptor;

	#[wasm_bindgen(method, setter, js_name = value)]
	pub(super) fn set_value(this: &GlobalDescriptor, value: &str);
}
