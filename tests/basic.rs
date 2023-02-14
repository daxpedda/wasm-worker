//! Tests basic functionality.

mod util;

use futures_util::future::{self, Either};
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::WorkerContext;

use self::util::{Flag, CLOSE_DURATION, SIGNAL_DURATION};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// [`wasm_worker::spawn()`].
#[wasm_bindgen_test]
async fn spawn() {
	let flag = Flag::new();

	wasm_worker::spawn({
		let flag = flag.clone();
		move |context| {
			flag.signal();
			context.close();
		}
	});

	flag.await;
}

/// [`wasm_worker::spawn_async()`].
#[wasm_bindgen_test]
async fn spawn_async() {
	let flag = Flag::new();

	wasm_worker::spawn_async({
		let flag = flag.clone();
		|context| async move {
			flag.signal();
			context.close();
		}
	});

	flag.await;
}

/// Nested workers.
#[wasm_bindgen_test]
async fn nested() {
	let inner = Flag::new();

	wasm_worker::spawn_async({
		let outer = inner.clone();
		|context| async move {
			let inner = Flag::new();

			wasm_worker::spawn({
				let outer = inner.clone();
				move |context| {
					outer.signal();
					context.close();
				}
			});

			inner.await;

			// Wait for nested worker to close.
			// See <https://bugs.chromium.org/p/chromium/issues/detail?id=1408115>.
			util::sleep(SIGNAL_DURATION).await;

			outer.signal();

			context.close();
		}
	});

	inner.await;
}

/// Nested workers in nested workers.
#[wasm_bindgen_test]
async fn nested_nested() {
	let inner = Flag::new();

	wasm_worker::spawn_async({
		let outer = inner.clone();
		|context| async move {
			let inner = Flag::new();

			wasm_worker::spawn_async({
				let outer = inner.clone();
				|context| async move {
					let inner = Flag::new();

					wasm_worker::spawn({
						let outer = inner.clone();
						move |context| {
							outer.signal();
							context.close();
						}
					});

					inner.await;

					// Wait for nested worker to close.
					// See <https://bugs.chromium.org/p/chromium/issues/detail?id=1408115>.
					util::sleep(SIGNAL_DURATION).await;

					outer.signal();

					context.close();
				}
			});

			inner.await;

			// Wait for nested worker to close.
			// See <https://bugs.chromium.org/p/chromium/issues/detail?id=1408115>.
			util::sleep(SIGNAL_DURATION).await;

			outer.signal();

			context.close();
		}
	});

	inner.await;
}

/// [`Worker::terminate()`](wasm_worker::Worker::terminate).
#[wasm_bindgen_test]
async fn terminate() {
	let request = Flag::new();
	let response = Flag::new();

	let worker = wasm_worker::spawn_async({
		let request = request.clone();
		let response = response.clone();

		|_| async move {
			// Worker will be terminated before the request signal is sent.
			request.await;
			response.signal();
		}
	});

	worker.terminate();
	request.signal();

	// The worker will never respond if terminated.
	let result = future::select(response, util::sleep(SIGNAL_DURATION)).await;
	assert!(matches!(result, Either::Right(((), _))));
}

/// [`WorkerContext::close()`].
#[wasm_bindgen_test]
async fn close() {
	let request = Flag::new();
	let response = Flag::new();

	wasm_worker::spawn_async({
		let request = request.clone();
		let response = response.clone();

		|context| async move {
			wasm_bindgen_futures::spawn_local(async move {
				request.await;
				response.signal();
			});

			context.close();
		}
	});

	// Wait for the worker to potentially stay alive.
	// This delay is intentionally big because `close()` can unfortunately take very
	// long.
	util::sleep(CLOSE_DURATION).await;

	request.signal();

	// The worker will never respond if terminated.
	let result = future::select(response, util::sleep(SIGNAL_DURATION)).await;
	assert!(matches!(result, Either::Right(((), _))));
}

/// [`WorkerContext::new()`].
#[wasm_bindgen_test]
async fn context() {
	let flag = Flag::new();

	wasm_worker::spawn_async({
		let flag = flag.clone();
		|context| async move {
			WorkerContext::new().unwrap();
			// Flag will never signal if `WorkerContext::new` panics.
			flag.signal();

			context.close();
		}
	});

	flag.await;
}

/// [`WorkerContext::new()`] fails outside worker.
#[wasm_bindgen_test]
fn context_fail() {
	assert!(WorkerContext::new().is_none());
}

/// [`WorkerContext::name()`].
#[wasm_bindgen_test]
async fn name() {
	let flag = Flag::new();

	wasm_worker::spawn_async({
		let flag = flag.clone();
		|context| async move {
			assert!(context.name().is_none());
			// Flag will never signal if `assert!` panics.
			flag.signal();

			context.close();
		}
	});

	flag.await;
}
