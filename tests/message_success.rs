#![cfg(test)]
#![cfg(all(target_family = "wasm", feature = "message"))]

use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::OffscreenCanvas;
use web_thread::web;
use web_thread::web::message::TransferableWrapper;
use web_thread::web::{JoinHandleExt, ScopeExt};

#[wasm_bindgen_test]
async fn spawn() {
	let canvas = OffscreenCanvas::new(1, 1).unwrap();
	web::spawn_with_message(
		|TransferableWrapper(canvas)| {
			assert_eq!(canvas.width(), 1);
			assert_eq!(canvas.height(), 1);
			async {}
		},
		TransferableWrapper(canvas.clone()),
	)
	.join_async()
	.await
	.unwrap();

	assert_eq!(canvas.width(), 0);
	assert_eq!(canvas.height(), 0);
}

#[wasm_bindgen_test]
async fn nested() {
	web::spawn_async(|| async {
		let canvas = OffscreenCanvas::new(1, 1).unwrap();
		web::spawn_with_message(
			|TransferableWrapper(canvas)| {
				assert_eq!(canvas.width(), 1);
				assert_eq!(canvas.height(), 1);
				async {}
			},
			TransferableWrapper(canvas.clone()),
		)
		.join_async()
		.await
		.unwrap();

		assert_eq!(canvas.width(), 0);
		assert_eq!(canvas.height(), 0);
	})
	.join_async()
	.await
	.unwrap();
}

#[wasm_bindgen_test]
async fn scope() {
	let canvas = OffscreenCanvas::new(1, 1).unwrap();
	web::scope_async(|scope| async {
		scope.spawn_with_message(
			|TransferableWrapper(canvas)| {
				assert_eq!(canvas.width(), 1);
				assert_eq!(canvas.height(), 1);
				async {}
			},
			TransferableWrapper(canvas.clone()),
		);
	})
	.await;

	assert_eq!(canvas.width(), 0);
	assert_eq!(canvas.height(), 0);
}
