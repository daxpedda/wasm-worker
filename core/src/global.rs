use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{DedicatedWorkerGlobalScope, Window};

pub(crate) enum Global {
	Window(Window),
	DedicatedWorker(DedicatedWorkerGlobalScope),
}

pub(crate) fn global_with<F: FnOnce(&Global) -> R, R>(f: F) -> R {
	thread_local! {
		static GLOBAL: Global = {
			#[wasm_bindgen]
			extern "C" {
				type Global;

				#[wasm_bindgen(method, getter, js_name = Window)]
				fn window(this: &Global) -> JsValue;

				#[wasm_bindgen(method, getter, js_name = DedicatedWorkerGlobalScope)]
				fn worker(this: &Global) -> JsValue;
			}

			let global: Global = js_sys::global().unchecked_into();

			if !global.window().is_undefined() {
				crate::Global::Window(global.unchecked_into())
			} else if !global.worker().is_undefined() {
				crate::Global::DedicatedWorker(global.unchecked_into())
			} else {
				panic!("only supported in a browser or web worker")
			}
		}
	}

	GLOBAL.with(f)
}
