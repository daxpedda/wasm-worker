//! Bindings to the JS API.

use js_sys::Promise;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use super::super::js::GlobalExt;

#[wasm_bindgen]
extern "C" {
	/// Returns the constructor of [`Worker`](https://developer.mozilla.org/en-US/docs/Web/API/Worker).
	#[wasm_bindgen(method, getter, js_name = Worker)]
	pub(super) fn worker(this: &GlobalExt) -> JsValue;

	/// Type of [`import.meta`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/import.meta).
	pub(super) type Meta;

	/// Returns [`import.meta`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/import.meta).
	#[wasm_bindgen(js_namespace = import, js_name = meta)]
	pub(super) static META: Meta;

	/// See [`import.meta.url`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/import.meta#url).
	#[wasm_bindgen(method, getter)]
	pub(super) fn url(this: &Meta) -> String;

	/// Returns [`Atomics.waitAsync`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Atomics/waitAsync).
	#[wasm_bindgen(js_namespace = Atomics, js_name = waitAsync)]
	pub(super) static HAS_WAIT_ASYNC: JsValue;

	/// Type for [`Atomics.waitAsync`s return value](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Atomics/waitAsync#return_value).
	pub(super) type WaitAsyncResult;

	/// [`async`] property of [`Atomics.waitAsync()`s return value].
	///
	/// [`async`]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Atomics/waitAsync#async
	/// [`Atomics.waitAsync()`s return value]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Atomics/waitAsync#return_value
	#[wasm_bindgen(method, getter, js_name = async)]
	pub(super) fn async_(this: &WaitAsyncResult) -> bool;

	/// [`value`] property of [`Atomics.waitAsync`s return value].
	///
	/// [`value`]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Atomics/waitAsync#value_2
	/// [`Atomics.waitAsync`s return value]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Atomics/waitAsync#return_value
	#[wasm_bindgen(method, getter)]
	pub(super) fn value(this: &WaitAsyncResult) -> Promise;
}
