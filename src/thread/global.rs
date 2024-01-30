//! Global context of each worker type.

use wasm_bindgen::JsCast;
use web_sys::{DedicatedWorkerGlobalScope, SharedWorkerGlobalScope, Window, WorkerGlobalScope};

use super::js::GlobalExt;

thread_local! {
	pub(super) static GLOBAL: Option<Global> = {
		let global: GlobalExt = js_sys::global().unchecked_into();

		if !global.window().is_undefined() {
			Some(Global::Window(global.unchecked_into()))
		} else if !global.dedicated_worker_global_scope().is_undefined() {
			Some(Global::Dedicated(global.unchecked_into()))
		} else if !global.shared_worker_global_scope().is_undefined() {
			Some(Global::Shared(global.unchecked_into()))
		} else if !global.service_worker_global_scope().is_undefined() {
			Some(Global::Service(global.unchecked_into()))
		} else if !global.audio_worklet_global_scope().is_undefined() {
			Some(Global::Worklet)
		} else {
			None
		}
	};
}

/// Global context.
pub(super) enum Global {
	/// [`Window`].
	Window(Window),
	/// [`DedicatedWorkerGlobalScope`].
	Dedicated(DedicatedWorkerGlobalScope),
	/// [`SharedWorkerGlobalScope`].
	Shared(SharedWorkerGlobalScope),
	/// Service worker.
	Service(WorkerGlobalScope),
	/// Worklet.
	Worklet,
}
