//! Polyfill for `Atomics.waitAsync`.

use std::cell::{Cell, RefCell};
use std::future;
use std::rc::Rc;
use std::task::{Poll, Waker};

use js_sys::Array;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::Worker;

use super::super::util::{MEMORY, MEMORY_ARRAY};
use super::js;
use super::url::ScriptUrl;

/// Arbitrary limited amount of workers to cache.
const POLYFILL_WORKER_CACHE: usize = 10;

/// Mimics the interface we need from [`Atomics`](js_sys::Atomics).
pub(super) struct Atomics;

impl Atomics {
	/// Mimics the interface we need from
	/// [`Atomics::wait_async`](js_sys::Atomics::wait_async).
	pub(super) async fn wait_async(value: &i32, check: i32) {
		thread_local! {
			static HAS_WAIT_ASYNC: bool = !js::Atomics::has_wait_async().is_undefined();
		}

		if HAS_WAIT_ASYNC.with(bool::clone) {
			let index: *const i32 = value;
			#[allow(clippy::as_conversions)]
			let index = index as u32 / 4;

			let result = MEMORY_ARRAY.with(|array| js::Atomics::wait_async(array, index, check));

			if result.async_() {
				JsFuture::from(result.value())
					.await
					.expect("`Promise` returned by `Atomics.waitAsync` should never throw");
			}
		} else {
			wait_async(value, check).await;
		}
	}
}

/// Polyfills [`Atomics::wait_async`](js_sys::Atomics::wait_async) if not
/// available.
async fn wait_async(value: &i32, check: i32) {
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

	let index: *const i32 = value;
	#[allow(clippy::as_conversions)]
	let index = index as u32 / 4;

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
