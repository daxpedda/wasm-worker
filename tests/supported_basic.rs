#![cfg(test)]

#[cfg(target_family = "wasm")]
use wasm_bindgen_test::wasm_bindgen_test;

#[cfg_attr(not(target_family = "wasm"), test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
fn available_parallelism() {
	web_thread::available_parallelism().unwrap();
}

#[cfg_attr(not(target_family = "wasm"), test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
fn thread() {
	let thread = web_thread::current();
	let _ = thread.id();
	let _ = thread.name();
}

#[cfg_attr(not(target_family = "wasm"), test)]
#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
fn panicking() {
	assert!(!web_thread::panicking());
}
