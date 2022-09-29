//! Basic example showing how to spawn workers.

#![allow(clippy::unwrap_used)]

use std::panic;
use std::time::Duration;

use futures_util::stream::FuturesUnordered;
use js_sys::Promise;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use wasm_worker::Error;
use web_sys::{console, DedicatedWorkerGlobalScope};

fn main() {}

/// Workaround for `#[wasm_bindgen(start)]` not supporting `async fn main()`.
#[wasm_bindgen(start)]
#[allow(clippy::future_not_send)]
pub async fn main_js() -> Result<(), JsValue> {
	panic::set_hook(Box::new(|panic_info| {
		wasm_worker::hook(panic_info);
	}));

	console::log_1(&"Start Example".into());

	// Spawning closures.
	for index in 0..10 {
		wasm_worker::spawn(move || console::log_1(&format!("Spawned closure: {index}").into()));
	}

	// Spawning `Future`s.
	for index in 0..10 {
		wasm_worker::spawn_async(move || async move {
			console::log_1(&format!("Spawned future: {index}").into());
		});
	}

	// Spawning long running worker.
	wasm_worker::spawn_async(|| async {
		let mut index = 0;

		loop {
			console::log_1(&format!("Spawned counter: {index}").into());
			sleep(Duration::from_secs(2)).await;
			index += 1;
		}
	});

	// Return values.
	let list = FuturesUnordered::new();

	for index in 0..10 {
		list.push(wasm_worker::spawn_async(move || async move { index }));
	}

	for future in list {
		console::log_1(&format!("Return value: {}", future.await?).into());
	}

	// Cancel workers.
	let mut list = Vec::with_capacity(10);

	for _ in 0..10 {
		list.push(wasm_worker::spawn_async(|| async {
			sleep(Duration::from_secs(1)).await;
			unreachable!("might be cancelled too slowly");
		}));
	}

	for handle in list {
		assert_eq!(handle.terminate(), Ok(None));
	}

	// Panic.
	assert!(matches!(
		wasm_worker::spawn(|| panic!("panicking worker")).await,
		Err(Error::Error(error)) if error.starts_with("panicked at 'panicking worker'"),
	));

	// Async panic.
	assert!(matches!(
		wasm_worker::spawn_async(|| async {
			panic!("panicking async worker");
		}).await,
		Err(Error::Error(error)) if error.starts_with("panicked at 'panicking async worker'"),
	));

	// Destructor.
	wasm_worker::spawn(|| {
		/// Struct to test if destructor is run.
		struct Test;

		impl Drop for Test {
			fn drop(&mut self) {
				console::log_1(&"Destructor run".into());
			}
		}

		let _test = Test;
	});

	// Actually make sure that terminated workers aren't panicking; they have an
	// built-in delay.
	sleep(Duration::from_secs(2)).await;

	Ok(())
}

/// Putting a worker to sleep.
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
