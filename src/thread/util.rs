//! Global context of each thread type.

use std::io::{Error, ErrorKind};
use std::sync::OnceLock;

use js_sys::Int32Array;
use js_sys::WebAssembly::{Memory, Module};
use wasm_bindgen::JsCast;
use web_sys::{DedicatedWorkerGlobalScope, SharedWorkerGlobalScope, Window, WorkerGlobalScope};

use super::js::GlobalExt;

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

/// Generates the appropriate error for an unsupported thready type.
pub(super) fn unsupported_global() -> Error {
	Error::new(
		ErrorKind::Unsupported,
		"encountered unsupported thread type",
	)
}

/// Returns [`true`] if `Worker` is supported in the global context this was
/// first called in.
pub(super) fn has_worker_support() -> bool {
	/// Caches worker support in the first context this is called in.
	static HAS_WORKER_SUPPORT: OnceLock<bool> = OnceLock::new();

	*HAS_WORKER_SUPPORT.get_or_init(|| {
		let global: GlobalExt = js_sys::global().unchecked_into();
		!global.worker().is_undefined()
	})
}

thread_local! {
	/// [`Memory`] of the Wasm module.
	pub(super) static MEMORY: Memory = wasm_bindgen::memory().unchecked_into();
	/// [`Memory`] of the Wasm module as a [`Int32Array`].
	pub(super) static MEMORY_ARRAY: Int32Array = Int32Array::new(&MEMORY.with(Memory::buffer));
	/// Wasm [`Module`].
	pub(super) static MODULE: Module = wasm_bindgen::module().unchecked_into();
}
