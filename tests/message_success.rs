#![cfg(test)]
#![cfg(all(target_family = "wasm", feature = "message"))]

use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::OffscreenCanvas;
use web_thread::web;
use web_thread::web::message::TransferableWrapper;

#[wasm_bindgen_test]
fn transfer() {
	let canvas = OffscreenCanvas::new(1, 1).unwrap();
	web::spawn_with_message(
		|TransferableWrapper(canvas)| {
			assert_eq!(canvas.width(), 0);
			assert_eq!(canvas.height(), 0);
			async {}
		},
		TransferableWrapper(canvas.clone()),
	);
	assert_eq!(canvas.width(), 0);
	assert_eq!(canvas.height(), 0);
}
