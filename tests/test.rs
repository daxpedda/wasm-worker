#![cfg(test)]

use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn basic() {
	let (sender, mut receiver) = futures_channel::oneshot::channel();
	sender.send(5).unwrap();
	assert_eq!(5, (&mut receiver).await.unwrap());
	receiver.await.unwrap();
}
