//! Bindings to the JS API.

use std::io::{Error, ErrorKind};

use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{DedicatedWorkerGlobalScope, Window};

#[wasm_bindgen]
extern "C" {
	type GlobalExt;

	#[wasm_bindgen(method, getter, js_name = Window)]
	fn window(this: &GlobalExt) -> JsValue;

	#[wasm_bindgen(method, getter, js_name = DedicatedWorkerGlobalScope)]
	fn dedicated_worker_global_scope(this: &GlobalExt) -> JsValue;

	#[wasm_bindgen(method, getter, js_name = AudioWorkletGlobalScope)]
	fn audio_worklet_global_scope(this: &GlobalExt) -> JsValue;
}

#[cfg(target_feature = "atomics")]
#[wasm_bindgen]
extern "C" {
	pub(super) type Meta;

	#[wasm_bindgen(js_namespace = import, js_name = meta)]
	pub(super) static META: Meta;

	#[wasm_bindgen(method, getter)]
	pub(super) fn url(this: &Meta) -> String;
}

/// Global context.
pub(super) enum Global {
	/// [`Window`].
	Window(Window),
	/// [`WorkerGlobalScope`].
	Worker(DedicatedWorkerGlobalScope),
	/// Worklet.
	Worklet,
}

thread_local! {
	pub(super) static GLOBAL: Option<Global> = {
		let global: GlobalExt = js_sys::global().unchecked_into();

		if !global.window().is_undefined() {
			Some(Global::Window(global.unchecked_into()))
		} else if !global.dedicated_worker_global_scope().is_undefined() {
			Some(Global::Worker(global.unchecked_into()))
		} else if !global.audio_worklet_global_scope().is_undefined() {
			Some(Global::Worklet)
		} else {
			None
		}
	};
}

/// Generates the appropriate error for an unsupported thready type.
pub(super) fn unsupported_global() -> Error {
	Error::new(
		ErrorKind::Unsupported,
		"encountered unsupported thread type",
	)
}
