use once_cell::unsync::Lazy;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Window, WorkerGlobalScope};

impl Global {
	pub(crate) fn new() -> Self {
		js_sys::global().unchecked_into()
	}
}

pub(crate) enum WindowOrWorker {
	Window(Window),
	Worker(WorkerGlobalScope),
}

impl WindowOrWorker {
	thread_local! {
		#[allow(clippy::use_self)]
		static THIS: Lazy<Option<WindowOrWorker>> = Lazy::new(|| {
			let global = Global::new();

			if !global.window().is_undefined() {
				Some(WindowOrWorker::Window(global.unchecked_into()))
			} else if !global.worker().is_undefined() {
				Some(WindowOrWorker::Worker(global.unchecked_into()))
			} else {
				None
			}
		});
	}

	pub(crate) fn with<R>(f: impl FnOnce(&Self) -> R) -> Option<R> {
		Self::THIS.with(|this| this.as_ref().map(f))
	}
}

#[wasm_bindgen]
extern "C" {
	#[allow(non_camel_case_types)]
	pub(crate) type Global;

	#[wasm_bindgen(method, getter, js_name = Window)]
	fn window(this: &Global) -> JsValue;

	#[wasm_bindgen(method, getter, js_name = WorkerGlobalScope)]
	fn worker(this: &Global) -> JsValue;

	#[wasm_bindgen(method, getter, js_name = AudioData)]
	pub(crate) fn audio_data(this: &Global) -> JsValue;

	#[wasm_bindgen(method, getter, js_name = OffscreenCanvas)]
	pub(crate) fn offscreen_canvas(this: &Global) -> JsValue;

	#[wasm_bindgen(method, getter, js_name = VideoFrame)]
	pub(crate) fn video_frame(this: &Global) -> JsValue;
}
