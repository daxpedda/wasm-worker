#![cfg(test)]
#![cfg(target_family = "wasm")]

use wasm_bindgen_test::wasm_bindgen_test;
use web_thread::web;
use web_time::Duration;

#[wasm_bindgen_test]
fn park_no_op() {
	web_thread::park();
	web_thread::park_timeout(Duration::from_secs(1));
	#[allow(deprecated)]
	web_thread::park_timeout_ms(1000);
}

#[wasm_bindgen_test]
#[should_panic = "current thread type cannot be blocked"]
fn sleep() {
	web_thread::sleep(Duration::from_secs(1));
}

#[wasm_bindgen_test]
#[should_panic = "current thread type cannot be blocked"]
fn sleep_ms() {
	#[allow(deprecated)]
	web_thread::sleep_ms(1000);
}

#[wasm_bindgen_test]
fn has_wait_support() {
	assert!(!web::has_wait_support());
}
