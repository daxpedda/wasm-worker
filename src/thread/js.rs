//! Bindings to the JS API.

use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen]
extern "C" {
	pub(super) type GlobalExt;

	#[wasm_bindgen(method, getter, js_name = Window)]
	pub(super) fn window(this: &GlobalExt) -> JsValue;

	#[wasm_bindgen(method, getter, js_name = DedicatedWorkerGlobalScope)]
	pub(super) fn dedicated_worker_global_scope(this: &GlobalExt) -> JsValue;

	#[wasm_bindgen(method, getter, js_name = SharedWorkerGlobalScope)]
	pub(super) fn shared_worker_global_scope(this: &GlobalExt) -> JsValue;

	#[wasm_bindgen(method, getter, js_name = ServiceWorkerGlobalScope)]
	pub(super) fn service_worker_global_scope(this: &GlobalExt) -> JsValue;

	#[wasm_bindgen(method, getter, js_name = AudioWorkletGlobalScope)]
	pub(super) fn audio_worklet_global_scope(this: &GlobalExt) -> JsValue;

	#[wasm_bindgen(js_name = crossOriginIsolated)]
	pub(super) static CROSS_ORIGIN_ISOLATED: bool;
}
