use once_cell::sync::OnceCell;
#[cfg(web_sys_unstable_apis)]
use web_sys::WritableStream;
#[cfg(not(web_sys_unstable_apis))]
use {wasm_bindgen::prelude::wasm_bindgen, wasm_bindgen::JsValue};

use super::super::MessageSupportError;
use crate::global::{Global, WindowOrWorker};

pub(in super::super) fn support() -> Result<bool, MessageSupportError> {
	#[cfg(not(web_sys_unstable_apis))]
	#[wasm_bindgen]
	extern "C" {
		#[wasm_bindgen(js_name = WritableStream)]
		type WritableStreamExt;

		#[wasm_bindgen(catch, constructor, js_class = "WritableStream")]
		fn new() -> Result<WritableStreamExt, JsValue>;
	}

	static SUPPORT: OnceCell<bool> = OnceCell::new();

	SUPPORT
		.get_or_try_init(|| {
			WindowOrWorker::with(|global| {
				if let WindowOrWorker::Worker(_) = global {
					if !Global::has_worker() {
						return Err(MessageSupportError);
					}
				}

				#[cfg(web_sys_unstable_apis)]
				let stream = WritableStream::new().unwrap();
				#[cfg(not(web_sys_unstable_apis))]
				let stream = WritableStreamExt::new().unwrap();

				Ok(super::test_support(&stream))
			})
			.unwrap_or(Err(MessageSupportError))
		})
		.copied()
}
