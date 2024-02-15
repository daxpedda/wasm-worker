//! Polyfill for `Atomics.waitAsync`.

use std::cell::{Cell, RefCell};
use std::future;
use std::rc::Rc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::task::{Poll, Waker};

use js_sys::{Array, Atomics};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::Worker;

use super::js::{self, WaitAsyncResult};
use super::url::ScriptUrl;
use super::{MEMORY, MEMORY_ARRAY};

/// Arbitrary limited amount of workers to cache.
const POLYFILL_WORKER_CACHE: usize = 10;

/// Mimics the interface we need from [`Atomics`].
pub(super) struct WaitAsync;

impl WaitAsync {
	/// Mimics the interface we need from [`Atomics::wait_async`].
	pub(super) async fn wait(value: &AtomicI32, check: i32) {
		thread_local! {
			static HAS_WAIT_ASYNC: bool = !js::HAS_WAIT_ASYNC.is_undefined();
		}

		// Short-circuit before having to go through FFI.
		if value.load(Ordering::Relaxed) != check {
			return;
		}

		let index = super::i32_to_buffer_index(value.as_ptr());

		if HAS_WAIT_ASYNC.with(bool::clone) {
			let result: WaitAsyncResult = MEMORY_ARRAY
				.with(|array| Atomics::wait_async(array, index, check))
				.expect("`Atomics.waitAsync` is not expected to fail")
				.unchecked_into();

			if result.async_() {
				JsFuture::from(result.value())
					.await
					.expect("`Promise` returned by `Atomics.waitAsync` should never throw");
			}
		} else {
			wait(index, check).await;
		}
	}
}

/// Polyfills [`Atomics::wait_async`] if not available.
async fn wait(index: u32, check: i32) {
	thread_local! {
		/// Object URL to the worker script.
		static URL: ScriptUrl = ScriptUrl::new(include_str!("wait_async.js"));
		/// Holds cached workers.
		static WORKERS: RefCell<Vec<Worker>> = const { RefCell::new(Vec::new()) };
	}

	let worker = WORKERS.with(|workers| {
		if let Some(worker) = workers.borrow_mut().pop() {
			return worker;
		}

		URL.with(|url| Worker::new(url.as_raw()))
			.expect("`new Worker()` is not expected to fail with a local script")
	});

	let finished = Rc::new(Cell::new(false));
	let waker: Rc<RefCell<Option<Waker>>> = Rc::new(RefCell::new(None));

	let onmessage_callback = Closure::once_into_js({
		let finished = Rc::clone(&finished);
		let waker = Rc::clone(&waker);
		let worker = worker.clone();

		move || {
			WORKERS.with(move |workers| {
				let mut workers = workers.borrow_mut();
				workers.push(worker);
				workers.truncate(POLYFILL_WORKER_CACHE);
			});

			finished.set(true);

			if let Some(waker) = waker.borrow_mut().take() {
				waker.wake();
			}
		}
	});
	worker.set_onmessage(Some(onmessage_callback.unchecked_ref()));

	let message =
		MEMORY.with(|memory| Array::of3(memory, &JsValue::from(index), &JsValue::from(check)));

	worker
		.post_message(&message)
		.expect("`Worker.postMessage` is not expected to fail without a `transfer` object");

	future::poll_fn(|cx| {
		if finished.get() {
			Poll::Ready(())
		} else {
			*waker.borrow_mut() = Some(cx.waker().clone());
			Poll::Pending
		}
	})
	.await;
}
