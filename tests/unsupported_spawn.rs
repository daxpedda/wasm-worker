#![cfg(test)]
#![cfg(target_family = "wasm")]

use wasm_bindgen_test::wasm_bindgen_test;
use web_thread::{web, Builder};

#[wasm_bindgen_test]
#[should_panic = "operation not supported on this platform without the atomics target feature"]
fn spawn() {
	web_thread::spawn(|| ());
}

#[wasm_bindgen_test]
#[should_panic = "operation not supported on this platform without the atomics target feature"]
fn builder() {
	Builder::new()
		.stack_size(usize::MAX)
		.name(String::from("test"))
		.spawn(|| ())
		.unwrap();
}

#[wasm_bindgen_test]
fn has_thread_support() {
	assert!(!web::has_spawn_support());
}
