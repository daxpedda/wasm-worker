#![cfg(target_family = "wasm")]

use wasm_bindgen_test::wasm_bindgen_test;
use web_thread::web;

#[wasm_bindgen_test]
#[should_panic = "`ScopeFuture` polled after completion"]
async fn scope_async() {
	let mut handle = Box::pin(web::scope_async(|_| async {}));
	(&mut handle).await;
	handle.await;
}
