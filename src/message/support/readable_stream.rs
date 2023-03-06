use once_cell::sync::OnceCell;
#[cfg(web_sys_unstable_apis)]
use web_sys::ReadableStream;
#[cfg(not(web_sys_unstable_apis))]
use {wasm_bindgen::prelude::wasm_bindgen, wasm_bindgen::JsValue};

use super::super::MessageSupportError;
use crate::global::{Global, GlobalContext};

pub(in super::super) fn support() -> Result<bool, MessageSupportError> {
	#[cfg(not(web_sys_unstable_apis))]
	#[wasm_bindgen]
	extern "C" {
		#[wasm_bindgen(js_name = ReadableStream)]
		type ReadableStreamExt;

		#[wasm_bindgen(catch, constructor, js_class = "ReadableStream")]
		fn new() -> Result<ReadableStreamExt, JsValue>;
	}

	static SUPPORT: OnceCell<bool> = OnceCell::new();

	SUPPORT
		.get_or_try_init(|| {
			GlobalContext::with(|global| match global {
				GlobalContext::Window(_) => Ok(()),
				GlobalContext::Worker(_) => {
					if Global::has_worker() {
						Ok(())
					} else {
						Err(MessageSupportError)
					}
				}
				GlobalContext::Worklet => Err(MessageSupportError),
			})?;

			#[cfg(web_sys_unstable_apis)]
			let stream = ReadableStream::new().unwrap();
			#[cfg(not(web_sys_unstable_apis))]
			let stream = ReadableStreamExt::new().unwrap();

			Ok(super::test_support(&stream))
		})
		.copied()
}
