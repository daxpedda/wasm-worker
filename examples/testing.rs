//! TODO

//#![feature(stdsimd)]
#![allow(
	clippy::indexing_slicing,
	clippy::missing_docs_in_private_items,
	clippy::unwrap_used,
	missing_docs
)]

//use core::arch::wasm32;
use std::borrow::Cow;
use std::future::Future;
use std::pin::Pin;
use std::task::{ready, Context, Poll};
use std::time::Duration;

use futures_util::FutureExt;
use js_sys::{ArrayBuffer, JsString, Promise};
use utf16_lit::utf16;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use wasm_worker::worklet::WorkletExt;
use wasm_worker::{worker, WorkletBuilder};
use web_sys::{console, DedicatedWorkerGlobalScope, OfflineAudioContext, Window};

#[wasm_bindgen(main)]
async fn main() {
	console_error_panic_hook::set_once();

	console::log_1(&worker::has_async_support().unwrap().await.into());

	let worker = wasm_worker::spawn(|context| {
		context.set_message_handler(|_, _| console::log_1(&"received".into()));
	});

	worker.transfer_messages([ArrayBuffer::new(1)]).unwrap();

	let audio =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();
	audio.add_wasm(|_| console::log_1(&lit_js!("audio"))).await;

	/*wasm_worker::spawn(|context| {
		wasm_bindgen_futures::spawn_local(async { console::log_1(&"from future".into()) });

		context.close();
		console::log_1(&"closed".into());

		let mut value = 0_i32;
		// SAFETY: This shouldn't be unsafe.
		unsafe { wasm32::memory_atomic_wait32(&mut value, value, -1) };
		unreachable!()
	});*/

	let audio =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();
	let worklet = WorkletBuilder::new()
		.message_handler(|_, message| {
			console::log_1(
				&format!("received audio message: {:?}", message.as_raw().data()).into(),
			);
		})
		.worklet_message_handler(|_, message| {
			console::log_1(&format_js!(
				"received window message: {:?}",
				message.as_raw().data()
			));
		})
		.add(Cow::Borrowed(&audio), move |context| {
			console::log_1(&lit_js!("audio 2"));
			context.transfer_messages([ArrayBuffer::new(1)]).unwrap();
		})
		.unwrap()
		.await;

	worklet.transfer_messages([ArrayBuffer::new(1)]).unwrap();

	sleep(Duration::from_secs(5)).await;
}

#[macro_export]
macro_rules! lit_js {
	($l:literal) => {
		JsString::from_char_code(&utf16!($l))
	};
}

#[macro_export]
macro_rules! format_js {
	($($t:tt)*) => {
		JsString::from_char_code(&format!($($t)*).encode_utf16().collect::<Vec<_>>())
	};
}

struct Sleep(JsFuture);

impl Future for Sleep {
	type Output = ();

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		ready!(self.0.poll_unpin(cx)).unwrap();
		Poll::Ready(())
	}
}

/// Sleeps for the given [`Duration`].
fn sleep(duration: Duration) -> Sleep {
	enum Global {
		Window(Window),
		DedicatedWorker(DedicatedWorkerGlobalScope),
	}

	thread_local! {
		/// Cached [`Global`].
		static GLOBAL: Global = {
			#[wasm_bindgen]
			extern "C" {
				type SleepGlobal;

				#[wasm_bindgen(method, getter, js_name = Window)]
				fn window(this: &SleepGlobal) -> JsValue;

				#[wasm_bindgen(method, getter, js_name = DedicatedWorkerGlobalScope)]
				fn worker(this: &SleepGlobal) -> JsValue;
			}

			let global: SleepGlobal = js_sys::global().unchecked_into();

			if !global.window().is_undefined() {
				Global::Window(global.unchecked_into())
			} else if !global.worker().is_undefined() {
				Global::DedicatedWorker(global.unchecked_into())
			} else {
				unreachable!("only supported in a browser or web worker")
			}
		};
	}

	let future =
		JsFuture::from(Promise::new(&mut |resolve, _| {
			let duration = duration.as_millis().try_into().unwrap();

			GLOBAL
				.with(|global| match global {
					Global::Window(window) => window
						.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, duration),
					Global::DedicatedWorker(worker) => worker
						.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, duration),
				})
				.unwrap();
		}));

	Sleep(future)
}
