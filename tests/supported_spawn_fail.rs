#![cfg(test)]
#![cfg(target_family = "wasm")]

use web_thread::web;
use web_thread::web::{JoinHandleExt, ScopedJoinHandleExt};

#[wasm_bindgen_test::wasm_bindgen_test]
#[should_panic = "`JoinHandleFuture` polled or created after completion"]
async fn join_async() {
	let mut handle = web_thread::spawn(|| ());
	handle.join_async().await.unwrap();
	let _ = handle.join_async().await;
}

#[wasm_bindgen_test::wasm_bindgen_test]
#[should_panic = "`JoinHandleFuture` polled or created after completion"]
async fn scope_join_async() {
	web::scope_async(|scope| async {
		let mut handle = scope.spawn(|| ());
		handle.join_async().await.unwrap();
		let _ = handle.join_async().await;
	})
	.await;
}
