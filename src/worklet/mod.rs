pub mod audio;
mod module;
mod support;

use std::error::Error;
use std::fmt::{Display, Formatter};

use wasm_bindgen::prelude::wasm_bindgen;

pub use self::audio::AudioWorkletExt;
pub use self::module::{WorkletModule, WorkletModuleError, WorkletModuleFuture};
pub use self::support::{has_import_support, ImportSupportFuture};

#[doc(hidden)]
#[allow(missing_debug_implementations, unreachable_pub)]
pub struct Data(Box<dyn 'static + FnOnce() + Send>);

#[doc(hidden)]
#[wasm_bindgen]
pub unsafe fn __wasm_worker_worklet_entry(data: *mut Data) {
	// SAFETY: Has to be a valid pointer to `Data`. We only call
	// `__wasm_worker_worklet_entry` from `worklet.js`. The data sent to it should
	// only come from `AudioWorkletExt::init_wasm()`.
	let data = *unsafe { Box::from_raw(data) };

	(data.0)();
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct WorkletInitError;

impl Display for WorkletInitError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "already initialized this worklet")
	}
}

impl Error for WorkletInitError {}
