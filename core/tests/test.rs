#![allow(unreachable_pub)]

use std::time::Duration;

use flag::Flag;
use futures_util::future::{self, Either};
use wasm_bindgen::{JsValue, ShimFormat};
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker_core::{Close, ScriptFormat, ScriptUrl, WorkerBuilder};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn basic() {
	let flag = Flag::new();

	wasm_worker_core::spawn({
		let flag = flag.clone();
		move |_| async move {
			flag.signal();
			Close::Yes
		}
	});

	flag.await;
}

#[wasm_bindgen_test]
async fn nested() {
	let outer_flag = Flag::new();

	wasm_worker_core::spawn({
		let outer_flag = outer_flag.clone();
		move |_| async move {
			let inner_flag = Flag::new();

			wasm_worker_core::spawn({
				let inner_flag = inner_flag.clone();
				move |_| async move {
					inner_flag.signal();
					Close::Yes
				}
			});

			inner_flag.await;

			// Wait for nested worker to close.
			// See <https://bugs.chromium.org/p/chromium/issues/detail?id=1408115>.
			sleep::sleep(Duration::from_millis(250)).await;

			outer_flag.signal();

			Close::Yes
		}
	});

	outer_flag.await;
}

#[wasm_bindgen_test]
async fn non_closing() {
	let signal_flag = Flag::new();
	let response_flag = Flag::new();

	let worker = wasm_worker_core::spawn({
		let signal_flag = signal_flag.clone();
		let response_flag = response_flag.clone();
		move |_| async move {
			wasm_bindgen_futures::spawn_local(async move {
				signal_flag.await;
				response_flag.signal();
			});

			Close::No
		}
	});

	signal_flag.signal();
	response_flag.await;
	worker.terminate();
}

#[wasm_bindgen_test]
async fn terminate() {
	let signal_flag = Flag::new();
	let response_flag = Flag::new();

	let worker = wasm_worker_core::spawn({
		let signal_flag = signal_flag.clone();
		let response_flag = response_flag.clone();
		move |_| async move {
			signal_flag.await;
			response_flag.signal();

			Close::Yes
		}
	});

	worker.terminate();
	signal_flag.signal();

	let result = future::select(response_flag, sleep::sleep(Duration::from_millis(250))).await;
	assert!(matches!(result, Either::Right(((), _))));
}

#[wasm_bindgen_test]
async fn builder_basic() -> Result<(), JsValue> {
	let flag = Flag::new();

	WorkerBuilder::new()?.spawn({
		let flag = flag.clone();
		move |_| async move {
			flag.signal();
			Close::Yes
		}
	});

	flag.await;

	Ok(())
}

#[wasm_bindgen_test]
async fn builder_name() -> Result<(), JsValue> {
	let flag = Flag::new();

	WorkerBuilder::new()?.name("test").spawn({
		let flag = flag.clone();
		move |context| async move {
			assert_eq!(context.name(), "test");

			flag.signal();
			Close::Yes
		}
	});

	flag.await;

	Ok(())
}

#[wasm_bindgen_test]
async fn builder_url() -> Result<(), JsValue> {
	let flag = Flag::new();

	let url = ScriptUrl::new(
		&wasm_bindgen::shim_url().unwrap(),
		match &wasm_bindgen::shim_format().unwrap() {
			ShimFormat::EsModule => ScriptFormat::EsModule,
			ShimFormat::NoModules { global_name } => ScriptFormat::Classic {
				global: global_name,
			},
			_ => unimplemented!(),
		},
	);

	WorkerBuilder::new_with_url(&url)?.spawn({
		let flag = flag.clone();
		move |_| async move {
			flag.signal();
			Close::Yes
		}
	});

	flag.await;

	Ok(())
}

mod sleep {
	use std::future::Future;
	use std::pin::Pin;
	use std::task::{ready, Context, Poll};
	use std::time::Duration;

	use futures_util::FutureExt;
	use js_sys::Promise;
	use wasm_bindgen::prelude::wasm_bindgen;
	use wasm_bindgen::{JsCast, JsValue};
	use wasm_bindgen_futures::JsFuture;
	use web_sys::{DedicatedWorkerGlobalScope, Window};

	enum Global {
		Window(Window),
		DedicatedWorker(DedicatedWorkerGlobalScope),
	}

	fn global_with<F: FnOnce(&Global) -> R, R>(f: F) -> R {
		thread_local! {
			static GLOBAL: Global = {
				#[wasm_bindgen]
				extern "C" {
					type JsGlobal;

					#[wasm_bindgen(method, getter, js_name = Window)]
					fn window(this: &JsGlobal) -> JsValue;

					#[wasm_bindgen(method, getter, js_name = DedicatedWorkerGlobalScope)]
					fn worker(this: &JsGlobal) -> JsValue;
				}

				let global: JsGlobal = js_sys::global().unchecked_into();

				if !global.window().is_undefined() {
					Global::Window(global.unchecked_into())
				} else if !global.worker().is_undefined() {
					Global::DedicatedWorker(global.unchecked_into())
				} else {
					panic!("only supported in a browser or web worker")
				}
			}
		}

		GLOBAL.with(f)
	}

	pub struct Sleep(JsFuture);

	impl Future for Sleep {
		type Output = ();

		fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
			ready!(self.0.poll_unpin(cx)).unwrap();
			Poll::Ready(())
		}
	}

	pub fn sleep(duration: Duration) -> Sleep {
		let future =
			JsFuture::from(Promise::new(&mut |resolve, _| {
				let duration = duration.as_millis().try_into().unwrap();

				global_with(|global| match global {
					Global::Window(window) => window
						.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, duration),
					Global::DedicatedWorker(worker) => worker
						.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, duration),
				})
				.unwrap();
			}));

		Sleep(future)
	}
}

mod flag {
	use std::future::Future;
	use std::pin::Pin;
	use std::sync::atomic::{AtomicBool, Ordering};
	use std::sync::Arc;
	use std::task::{Context, Poll};

	use futures_util::task::AtomicWaker;

	struct Inner {
		waker: AtomicWaker,
		set: AtomicBool,
	}

	#[derive(Clone)]
	pub struct Flag(Arc<Inner>);

	impl Flag {
		pub fn new() -> Self {
			Self(Arc::new(Inner {
				waker: AtomicWaker::new(),
				set: AtomicBool::new(false),
			}))
		}

		pub fn signal(&self) {
			self.0.set.store(true, Ordering::Relaxed);
			self.0.waker.wake();
		}
	}

	impl Future for Flag {
		type Output = ();

		fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
			if self.0.set.load(Ordering::Relaxed) {
				return Poll::Ready(());
			}

			self.0.waker.register(cx.waker());

			if self.0.set.load(Ordering::Relaxed) {
				Poll::Ready(())
			} else {
				Poll::Pending
			}
		}
	}
}
