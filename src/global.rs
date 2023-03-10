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

#[cfg_attr(
	not(any(feature = "message", feature = "worklet")),
	allow(unused_tuple_struct_fields)
)]
pub(crate) enum GlobalContext {
	Window(Window),
	Worker(WorkerGlobalScope),
	Worklet,
}

impl GlobalContext {
	thread_local! {
		#[allow(clippy::use_self)]
		static THIS: Lazy<GlobalContext> = Lazy::new(|| {
			Global::with(|global| {
				if !global.window().is_undefined() {
					GlobalContext::Window(global.clone().unchecked_into())
				} else if !global.worker_global_scope().is_undefined() {
					GlobalContext::Worker(global.clone().unchecked_into())
				} else if !global.audio_worklet_global_scope().is_undefined() {
					GlobalContext::Worklet
				} else {
					panic!("expected to be in a window, worker or audio worklet")
				}
			})
		});
	}

	pub(crate) fn with<R>(f: impl FnOnce(&Self) -> R) -> R {
		Self::THIS.with(|this| f(this.deref()))
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

	#[wasm_bindgen(method, getter, js_name = AudioWorkletGlobalScope)]
	fn audio_worklet_global_scope(this: &Global) -> JsValue;

	#[wasm_bindgen(method, getter, js_name = Worker)]
	fn worker(this: &Global) -> JsValue;

	#[wasm_bindgen(method, getter, js_name = AudioData)]
	pub(crate) fn audio_data(this: &Global) -> JsValue;

	#[wasm_bindgen(method, getter, js_name = OffscreenCanvas)]
	pub(crate) fn offscreen_canvas(this: &Global) -> JsValue;

	#[wasm_bindgen(method, getter, js_name = VideoFrame)]
	pub(crate) fn video_frame(this: &Global) -> JsValue;
}
