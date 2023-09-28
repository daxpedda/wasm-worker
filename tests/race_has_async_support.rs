//! Tests behavior of [`worker::has_async_support()`].

#![cfg(test)]
#![allow(clippy::missing_assert_message)]

use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::worker;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// [`worker::has_async_support()`].
#[wasm_bindgen_test]
async fn test() {
	#[wasm_bindgen]
	extern "C" {
		type Atomics;

		#[wasm_bindgen(static_method_of = Atomics, js_name = waitAsync, getter)]
		fn test_wait_async() -> JsValue;
	}

	if !Atomics::test_wait_async().is_undefined() {
		assert_eq!(
			worker::has_async_support().unwrap().into_inner(),
			Some(true)
		);
		return;
	}

	let mut future_1 = worker::has_async_support().unwrap();
	assert!(future_1.into_inner().is_none());

	let mut future_2 = worker::has_async_support().unwrap();
	assert!(future_2.into_inner().is_none());
	assert!(future_1.into_inner().is_none());

	future_2.await;
	assert!(future_1.into_inner().is_some());

	let mut future_3 = worker::has_async_support().unwrap();
	assert!(future_3.into_inner().is_some());
}
