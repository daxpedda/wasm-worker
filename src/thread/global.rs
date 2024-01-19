//! Global context of each thread type.

use std::io::{Error, ErrorKind};

use wasm_bindgen::JsCast;
use web_sys::{DedicatedWorkerGlobalScope, Window};

use super::js::GlobalExt;

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
