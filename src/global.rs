use std::ops::Deref;

use once_cell::unsync::Lazy;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Window, WorkerGlobalScope};

impl Global {
	thread_local! {
		#[allow(clippy::use_self)]
		static GLOBAL: Lazy<Global> = Lazy::new(|| js_sys::global().unchecked_into());
	}

	pub(crate) fn with<R>(f: impl FnOnce(&Self) -> R) -> R {
		Self::GLOBAL.with(|global| f(global))
	}

	pub(crate) fn has_worker() -> bool {
		thread_local! {
			static WORKER: Lazy<bool> = Lazy::new(|| !Global::with(Global::worker).is_undefined())
		}

		WORKER.with(|worker| *worker.deref())
	}
}

#[cfg_attr(not(feature = "worklet"), allow(unused_tuple_struct_fields))]
pub(crate) enum WindowOrWorker {
	Window(Window),
	Worker(WorkerGlobalScope),
}

impl WindowOrWorker {
	thread_local! {
		#[allow(clippy::use_self)]
		static THIS: Lazy<Option<WindowOrWorker>> = Lazy::new(|| {
			Global::with(|global| {
				if !global.window().is_undefined() {
					Some(WindowOrWorker::Window(global.clone().unchecked_into()))
				} else if !global.worker_global_scope().is_undefined() {
					Some(WindowOrWorker::Worker(global.clone().unchecked_into()))
				} else {
					None
				}
			})
		});
	}

	pub(crate) fn with<R>(f: impl FnOnce(&Self) -> R) -> Option<R> {
		Self::THIS.with(|this| this.as_ref().map(f))
	}
}

#[wasm_bindgen]
extern "C" {
	#[derive(Clone)]
	pub(crate) type Global;

	#[wasm_bindgen(method, getter, js_name = Window)]
	fn window(this: &Global) -> JsValue;

	#[wasm_bindgen(method, getter, js_name = WorkerGlobalScope)]
	fn worker_global_scope(this: &Global) -> JsValue;

	#[wasm_bindgen(method, getter, js_name = Worker)]
	fn worker(this: &Global) -> JsValue;

	#[wasm_bindgen(method, getter, js_name = AudioData)]
	pub(crate) fn audio_data(this: &Global) -> JsValue;

	#[wasm_bindgen(method, getter, js_name = OffscreenCanvas)]
	pub(crate) fn offscreen_canvas(this: &Global) -> JsValue;

	#[wasm_bindgen(method, getter, js_name = VideoFrame)]
	pub(crate) fn video_frame(this: &Global) -> JsValue;
}
