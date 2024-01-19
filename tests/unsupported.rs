#![cfg(test)]
#![cfg(target_family = "wasm")]

use wasm_bindgen_test::wasm_bindgen_test;
use web_time::Duration;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

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
