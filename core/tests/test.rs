#![cfg(test)]

use futures_channel::oneshot;
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker_core::Close;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn basic() {
	let (sender, receiver) = oneshot::channel();

	wasm_worker_core::spawn(|| async {
		sender.send(()).unwrap();
		Close::Yes
	});

	receiver.await.unwrap();
}

#[wasm_bindgen_test]
async fn nested() {
	let (outer_sender, outer_receiver) = oneshot::channel();

	wasm_worker_core::spawn(|| async {
		/*let (inner_sender, inner_receiver) = oneshot::channel();

		wasm_worker_core::spawn(|| async {
			inner_sender.send(()).unwrap();
			Close::No
		});

		inner_receiver.await.unwrap();*/

		web_sys::console::log_1(&"test 1".into());

		wasm_worker_core::spawn(|| async {
			web_sys::console::log_1(&"test 2".into());
			outer_sender.send(()).unwrap();
			Close::Yes
		});

		Close::No
	});

	outer_receiver.await.unwrap();
}
