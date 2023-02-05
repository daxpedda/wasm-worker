use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::DedicatedWorkerGlobalScope;

pub(crate) enum Global {
	DedicatedWorker(DedicatedWorkerGlobalScope),
}

pub(crate) fn global_with<F: FnOnce(Option<&Global>) -> R, R>(f: F) -> R {
	thread_local! {
		static GLOBAL: Option<Global> = {
			#[wasm_bindgen]
			extern "C" {
				type Global;

				#[wasm_bindgen(method, getter, js_name = DedicatedWorkerGlobalScope)]
				fn worker(this: &Global) -> JsValue;
			}

			let global: Global = js_sys::global().unchecked_into();

			if global.worker().is_undefined() {
				None
			} else {
				Some(crate::Global::DedicatedWorker(global.unchecked_into()))
			}
		}
	}

	GLOBAL.with(|global| f(global.as_ref()))
}
