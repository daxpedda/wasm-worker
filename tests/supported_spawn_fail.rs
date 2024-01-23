#![cfg(test)]
#![cfg(target_family = "wasm")]

use web_thread::web::JoinHandleExt;

#[wasm_bindgen_test::wasm_bindgen_test]
#[should_panic = "`JoinHandleFuture` polled or created after completion"]
async fn join_async() {
	let mut handle = web_thread::spawn(|| ());
	handle.join_async().await.unwrap();
	let _ = handle.join_async().await;
}
