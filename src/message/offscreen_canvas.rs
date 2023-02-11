use once_cell::sync::Lazy;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use web_sys::OffscreenCanvas;

use super::{util, SupportError};

pub(super) fn support() -> Result<(), SupportError> {
	static SUPPORT: Lazy<Result<(), SupportError>> = Lazy::new(|| {
		#[wasm_bindgen]
		extern "C" {
			type OffscreenCanvasGlobal;

			#[wasm_bindgen(method, getter, js_name = OffscreenCanvas)]
			fn offscreen_canvas(this: &OffscreenCanvasGlobal) -> JsValue;
		}

		let global: OffscreenCanvasGlobal = js_sys::global().unchecked_into();

		if global.offscreen_canvas().is_undefined() {
			return Err(SupportError::Unsupported);
		}

		let canvas = OffscreenCanvas::new(1, 0).unwrap_throw();

		util::has_support(&canvas)
	});

	*SUPPORT
}
