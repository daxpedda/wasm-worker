//! Basic example showing how to spawn threads.

#![allow(clippy::unwrap_used)]

use std::panic;
use std::time::Duration;

use js_sys::Promise;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{console, DedicatedWorkerGlobalScope};

fn main() {
	panic::set_hook(Box::new(console_error_panic_hook::hook));

	// Spawning closures.
	for index in 0..10 {
		wasm_thread::spawn(move || console::log_1(&format!("Spawned closure: {index}").into()));
	}

	// Spawning `Future`s.
	for index in 0..10 {
		wasm_thread::spawn_async(move || async move {
			console::log_1(&format!("Spawned future: {index}").into());
		});
	}

	// Spawning long running thread.
	wasm_thread::spawn_async(|| async {
		let mut index = 0;

		loop {
			console::log_1(&format!("Spawned counter: {index}").into());
			sleep(Duration::from_secs(2)).await;
			index += 1;
		}
	});
}

/// Putting a thread to sleep.
#[allow(clippy::future_not_send)]
async fn sleep(duration: Duration) {
	JsFuture::from(Promise::new(&mut |resolve, _| {
		let global: DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();
		global
			.set_timeout_with_callback_and_timeout_and_arguments_0(
				&resolve,
				duration.as_millis().try_into().unwrap(),
			)
			.unwrap();
	}))
	.await
	.unwrap();
}
