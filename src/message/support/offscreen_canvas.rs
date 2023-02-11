use once_cell::sync::Lazy;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use web_sys::OffscreenCanvas;

use super::super::SupportError;

pub(in super::super) fn support() -> Result<(), SupportError> {
	static SUPPORT: Lazy<Result<(), SupportError>> = Lazy::new(|| {
		#[wasm_bindgen]
		extern "C" {
			#[allow(non_camel_case_types)]
			type __wasm_worker_OffscreenCanvasGlobal;

			#[wasm_bindgen(method, getter, js_name = OffscreenCanvas)]
			fn offscreen_canvas(this: &__wasm_worker_OffscreenCanvasGlobal) -> JsValue;
		}

		let global: __wasm_worker_OffscreenCanvasGlobal = js_sys::global().unchecked_into();

		if global.offscreen_canvas().is_undefined() {
			return Err(SupportError::Unsupported);
		}

		let canvas = OffscreenCanvas::new(1, 0).unwrap_throw();

		super::test_support(&canvas)
	});

	*SUPPORT
}
