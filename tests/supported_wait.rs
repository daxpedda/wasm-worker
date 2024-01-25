#![cfg(test)]

#[cfg(not(target_family = "wasm"))]
use std::time;

use time::{Duration, Instant};
#[cfg(target_family = "wasm")]
use wasm_bindgen_test::wasm_bindgen_test;
#[cfg(target_family = "wasm")]
use web_time as time;

#[cfg_attr(not(target_family = "wasm"), test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
fn park() {
	let start = Instant::now();

	let thread = web_thread::current();
	thread.unpark();

	web_thread::park();
	web_thread::park_timeout(Duration::from_secs(1));
	#[allow(deprecated)]
	web_thread::park_timeout_ms(1000);

	let elapsed = start.elapsed();
	assert!(elapsed.as_secs() >= 2, "time: {elapsed:?}");
}

#[cfg_attr(not(target_family = "wasm"), test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
fn sleep() {
	let start = Instant::now();

	web_thread::sleep(Duration::from_secs(1));
	#[allow(deprecated)]
	web_thread::sleep_ms(1000);

	let elapsed = start.elapsed();
	assert!(elapsed.as_secs() >= 2, "time: {elapsed:?}");
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen_test::wasm_bindgen_test]
#[allow(clippy::absolute_paths)]
fn has_wait_support() {
	assert!(web_thread::web::has_wait_support());
}
