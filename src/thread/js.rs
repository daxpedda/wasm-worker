//! Bindings to the JS API.

use js_sys::{Function, Promise};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use web_sys::{AbortSignal, Window};

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

	#[wasm_bindgen(method, getter, js_name = WorkerGlobalScope)]
	pub(super) fn worker_global_scope(this: &GlobalExt) -> JsValue;

	#[wasm_bindgen(method, getter, js_name = WorkletGlobalScope)]
	pub(super) fn worklet_global_scope(this: &GlobalExt) -> JsValue;

	#[wasm_bindgen(extends = Window)]
	pub(super) type WindowExt;

	#[wasm_bindgen(method, getter, js_name = requestIdleCallback)]
	pub(super) fn has_request_idle_callback(this: &WindowExt) -> JsValue;

	#[wasm_bindgen]
	pub(super) type WindowOrWorkerExt;

	#[wasm_bindgen(method, getter, js_name = scheduler)]
	pub(super) fn has_scheduler(this: &WindowOrWorkerExt) -> JsValue;

	#[wasm_bindgen(method, getter)]
	pub(super) fn scheduler(this: &WindowOrWorkerExt) -> Scheduler;

	pub(super) type Scheduler;

	#[wasm_bindgen(method, js_name = postTask)]
	pub(super) fn post_task_with_options(
		this: &Scheduler,
		callback: &Function,
		options: &SchedulerPostTaskOptions,
	) -> Promise;

	pub(super) type SchedulerPostTaskOptions;

	#[wasm_bindgen(method, setter, js_name = signal)]
	pub(super) fn set_signal(this: &SchedulerPostTaskOptions, signal: &AbortSignal);

	#[wasm_bindgen(method, setter, js_name = priority)]
	pub(super) fn set_priority(this: &SchedulerPostTaskOptions, priority: TaskPriority);

	#[wasm_bindgen(js_name = crossOriginIsolated)]
	pub(super) static CROSS_ORIGIN_ISOLATED: bool;
}

#[wasm_bindgen]
pub(super) enum TaskPriority {
	UserBlocking = "user-blocking",
	UserVisible = "user-visible",
	Background = "background",
}
