use once_cell::sync::OnceCell;
#[cfg(web_sys_unstable_apis)]
use web_sys::TransformStream;
#[cfg(not(web_sys_unstable_apis))]
use {wasm_bindgen::prelude::wasm_bindgen, wasm_bindgen::JsValue};

use super::super::MessageSupportError;
use crate::global::{Global, GlobalContext};

pub(in super::super) fn support() -> Result<bool, MessageSupportError> {
	#[cfg(not(web_sys_unstable_apis))]
	#[wasm_bindgen]
	extern "C" {
		#[wasm_bindgen(js_name = TransformStream)]
		type TransformStreamExt;

		#[wasm_bindgen(catch, constructor, js_class = "TransformStream")]
		fn new() -> Result<TransformStreamExt, JsValue>;
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
			let stream = TransformStream::new().unwrap();
			#[cfg(not(web_sys_unstable_apis))]
			let stream = TransformStreamExt::new().unwrap();

			Ok(super::test_support(&stream))
		})
		.copied()
}
