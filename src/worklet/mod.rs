pub mod audio;
mod module;

use std::error::Error;
use std::fmt::{Display, Formatter};

use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsCast;
use web_sys::WorkletGlobalScope;

pub use self::audio::AudioWorkletExt;
pub use self::module::{
	ImportSupportFuture, WorkletModule, WorkletModuleError, WorkletModuleFuture,
};

#[doc(hidden)]
#[allow(missing_debug_implementations, unreachable_pub)]
pub struct Data {
	id: usize,
	task: Box<dyn 'static + FnOnce(WorkletGlobalScope, usize) + Send>,
}

#[doc(hidden)]
#[wasm_bindgen]
pub unsafe fn __wasm_worker_worklet_entry(data: *mut Data) {
	// SAFETY: Has to be a valid pointer to `Data`. We only call
	// `__wasm_worker_worklet_entry` from `worklet.js`. The data sent to it should
	// only come from `AudioWorkletExt::init_wasm()`.
	let data = *unsafe { Box::from_raw(data) };

	let global = js_sys::global().unchecked_into();

	(data.task)(global, data.id);
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct WorkletInitError;

impl Display for WorkletInitError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "already initialized this worklet")
	}
}

impl Error for WorkletInitError {}
