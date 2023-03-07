#![feature(stdsimd)]
#![allow(clippy::missing_docs_in_private_items, missing_docs)]

use core::arch::wasm32;

use js_sys::{ArrayBuffer, JsString};
use utf16_lit::utf16;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use wasm_worker::worker;
use wasm_worker::worklet::{WorkletExt, WorkletUrl};
use web_sys::{console, AudioContext, Response};

#[wasm_bindgen(main)]
async fn main() {
	console_error_panic_hook::set_once();

	console::log_1(&worker::has_async_support().unwrap().await.into());

	console::log_1(&WorkletUrl::has_import_support().await.into());

	let worker = wasm_worker::spawn(|context| {
		context.set_message_handler(|_, _| console::log_1(&"received".into()));
	});

	worker.transfer_messages([ArrayBuffer::new(1)]).unwrap();

	let audio = AudioContext::new().unwrap();
	audio
		.add_wasm(|_| {
			let string = JsString::from_char_code(&utf16!("audio"));
			console::log_1(&string);
		})
		.unwrap()
		.await
		.unwrap();

	let module = WorkletUrl::default().await.unwrap();
	let promise = web_sys::window().unwrap().fetch_with_str(module.as_raw());
	let response: Response = JsFuture::from(promise).await.unwrap().unchecked_into();
	let promise = response.text().unwrap();
	let text: JsString = JsFuture::from(promise).await.unwrap().unchecked_into();

	console::log_1(&text);

	wasm_worker::spawn(|context| {
		wasm_bindgen_futures::spawn_local(async { console::log_1(&"from future".into()) });

		context.close();
		console::log_1(&"closed".into());

		let mut value = 0_i32;
		// SAFETY: This shouldn't be unsafe.
		unsafe { wasm32::memory_atomic_wait32(&mut value, value, -1) };
		unreachable!()
	});
}
