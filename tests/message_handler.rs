//! Tests functionality around setting and clearing message handlers in
//! [`WorkerBuilder`], [`WorkerHandle`](wasm_worker::WorkerHandle) and
//! [`WorkerContext`](wasm_worker::WorkerContext).

mod util;

use futures_util::future::{self, Either};
use js_sys::{ArrayBuffer, Uint8Array};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::{Close, Message, WorkerBuilder};

use self::util::{Flag, SIGNAL_DURATION};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// [`WorkerBuilder::message_handler()`] with
/// [`WorkerHandle::clear_message_handler()`](wasm_worker::WorkerHandle::clear_message_handler).
#[wasm_bindgen_test]
async fn builder_clear_message_handler() -> Result<(), JsValue> {
	assert!(Message::has_array_buffer_support().is_ok());

	let request = Flag::new();
	let response = Flag::new();

	let worker = WorkerBuilder::new()?
		.message_handler({
			let response = response.clone();
			move |_, _| response.signal()
		})
		.spawn_async({
			let request = request.clone();
			|context| async move {
				request.await;
				context.transfer_messages([ArrayBuffer::new(1)]).unwrap();

				Close::No
			}
		});

	worker.clear_message_handler();
	request.signal();

	// The message handler will never respond if cleared.
	let result = future::select(response, util::sleep(SIGNAL_DURATION)).await;
	assert!(matches!(result, Either::Right(((), _))));

	worker.terminate();

	Ok(())
}

/// [`WorkerHandle::set_message_handler()`](wasm_worker::WorkerHandle::set_message_handler) with
/// [`WorkerHandle::clear_message_handler()`](wasm_worker::WorkerHandle::clear_message_handler).
#[wasm_bindgen_test]
async fn handler_clear_message_handler() {
	assert!(Message::has_array_buffer_support().is_ok());

	let request = Flag::new();
	let response = Flag::new();

	let worker = wasm_worker::spawn_async({
		let request = request.clone();
		|context| async move {
			request.await;
			context.transfer_messages([ArrayBuffer::new(1)]).unwrap();

			Close::No
		}
	});

	worker.set_message_handler({
		let response = response.clone();
		move |_, _| response.signal()
	});
	worker.clear_message_handler();
	request.signal();

	// The message handler will never respond if cleared.
	let result = future::select(response, util::sleep(SIGNAL_DURATION)).await;
	assert!(matches!(result, Either::Right(((), _))));

	worker.terminate();
}

/// [`WorkerContext::set_message_handler()`](wasm_worker::WorkerContext::set_message_handler) with
/// [`WorkerContext::clear_message_handler()`](wasm_worker::WorkerContext::clear_message_handler).
#[wasm_bindgen_test]
async fn context_clear_message_handler() {
	assert!(Message::has_array_buffer_support().is_ok());

	let request = Flag::new();
	let response = Flag::new();

	let worker = wasm_worker::spawn_async({
		let request = request.clone();
		let response = response.clone();
		|context| async move {
			context.set_message_handler(move |_, _| response.signal());
			context.clear_message_handler();
			request.signal();

			Close::No
		}
	});

	request.await;
	worker.transfer_messages([ArrayBuffer::new(1)]).unwrap();

	// The message handler will never respond if cleared.
	let result = future::select(response, util::sleep(SIGNAL_DURATION)).await;
	assert!(matches!(result, Either::Right(((), _))));

	worker.terminate();
}

/// [`WorkerBuilder::message_handler()`] with
/// [`WorkerHandle::builder_has_message_handler()`](wasm_worker::WorkerHandle::builder_has_message_handler).
#[wasm_bindgen_test]
fn builder_has_message_handler() -> Result<(), JsValue> {
	let worker = WorkerBuilder::new()?
		.message_handler(|_, _| ())
		.spawn(|_| Close::Yes);
	assert!(worker.has_message_handler());
	worker.clear_message_handler();
	assert!(!worker.has_message_handler());

	Ok(())
}

/// [`WorkerHandle::set_message_handler()`](wasm_worker::WorkerHandle::set_message_handler) with
/// [`WorkerHandle::has_message_handler()`](wasm_worker::WorkerHandle::has_message_handler).
#[wasm_bindgen_test]
fn handler_has_message_handler() {
	let worker = wasm_worker::spawn(|_| Close::Yes);

	assert!(!worker.has_message_handler());
	worker.set_message_handler(|_, _| ());
	assert!(worker.has_message_handler());
	worker.clear_message_handler();
	assert!(!worker.has_message_handler());
}

/// [`WorkerContext::has_message_handler()`](wasm_worker::WorkerContext::has_message_handler).
#[wasm_bindgen_test]
async fn context_has_message_handler() {
	let flag = Flag::new();

	wasm_worker::spawn({
		let flag = flag.clone();
		move |context| {
			assert!(!context.has_message_handler());
			context.set_message_handler(|_, _| ());
			assert!(context.has_message_handler());
			context.clear_message_handler();
			assert!(!context.has_message_handler());

			// Flag will never signal if `assert!`s panic.
			flag.signal();

			Close::Yes
		}
	});

	flag.await;
}

/// [`WorkerBuilder::message_handler_async()`].
#[wasm_bindgen_test]
async fn builder_async_message_handler() -> Result<(), JsValue> {
	assert!(Message::has_array_buffer_support().is_ok());

	let flag = Flag::new();

	let _worker = WorkerBuilder::new()?
		.message_handler_async({
			let flag = flag.clone();
			move |_, _| {
				let flag = flag.clone();
				async move { flag.signal() }
			}
		})
		.spawn({
			|context| {
				context.transfer_messages([ArrayBuffer::new(1)]).unwrap();

				Close::Yes
			}
		});

	flag.await;

	Ok(())
}

/// [`WorkerHandle::set_message_handler_async()`](wasm_worker::WorkerHandle::set_message_handler_async).
#[wasm_bindgen_test]
async fn handler_async_message_handler() {
	assert!(Message::has_array_buffer_support().is_ok());

	let request = Flag::new();
	let response = Flag::new();

	let worker = wasm_worker::spawn_async({
		let request = request.clone();
		|context| async move {
			request.await;
			context.transfer_messages([ArrayBuffer::new(1)]).unwrap();

			Close::Yes
		}
	});

	worker.set_message_handler_async({
		let response = response.clone();
		move |_, _| {
			let response = response.clone();
			async move { response.signal() }
		}
	});
	request.signal();

	response.await;
}

/// [`WorkerContext::set_message_handler_async()`](wasm_worker::WorkerContext::set_message_handler_async).
#[wasm_bindgen_test]
async fn context_async_message_handler() {
	assert!(Message::has_array_buffer_support().is_ok());

	let request = Flag::new();
	let response = Flag::new();

	let worker = wasm_worker::spawn_async({
		let request = request.clone();
		let response = response.clone();
		|context| async move {
			context.set_message_handler_async(move |_, _| {
				let response = response.clone();
				async move { response.signal() }
			});
			request.signal();

			Close::No
		}
	});

	request.await;
	worker.transfer_messages([ArrayBuffer::new(1)]).unwrap();

	response.await;

	worker.terminate();
}

/// Multiple messages in
/// [`WorkerHandle::transfer_messages()`](wasm_worker::WorkerHandle::transfer_messages).
#[wasm_bindgen_test]
async fn handler_multi_message() {
	assert!(Message::has_array_buffer_support().is_ok());

	let request = Flag::new();
	let response = Flag::new();

	let worker = wasm_worker::spawn_async({
		let request = request.clone();
		let response = response.clone();

		|context| async move {
			context.set_message_handler(move |_, event| {
				let messages = event.messages();
				assert_eq!(messages.len(), 3);

				for (index, message) in (1_u8..).zip(messages) {
					let buffer: ArrayBuffer = message.serialize_as().unwrap();

					let array = Uint8Array::new(&buffer);
					assert_eq!(buffer.byte_length(), index.into());
					let mut output = [0; 3];
					array.copy_to(&mut output[..index.into()]);
					assert!(output[..index.into()].iter().all(|value| *value == index));
				}

				response.signal();
			});

			request.signal();

			Close::No
		}
	});

	request.await;

	let buffer_1 = ArrayBuffer::new(1);
	let array = Uint8Array::new(&buffer_1);
	array.copy_from(&[1]);

	let buffer_2 = ArrayBuffer::new(2);
	let array = Uint8Array::new(&buffer_2);
	array.copy_from(&[2; 2]);

	let buffer_3 = ArrayBuffer::new(3);
	let array = Uint8Array::new(&buffer_3);
	array.copy_from(&[3; 3]);

	worker
		.transfer_messages([buffer_1, buffer_2, buffer_3])
		.unwrap();

	response.await;

	worker.terminate();
}

/// Multiple messages in
/// [`WorkerContext::transfer_messages()`](wasm_worker::WorkerContext::transfer_messages).
#[wasm_bindgen_test]
async fn context_multi_message() -> Result<(), JsValue> {
	assert!(Message::has_array_buffer_support().is_ok());

	let flag = Flag::new();

	let _worker = WorkerBuilder::new()?
		.message_handler({
			let flag = flag.clone();
			move |_, event| {
				let messages = event.messages();

				assert_eq!(messages.len(), 3);

				for (index, message) in (1_u8..).zip(messages) {
					let buffer: ArrayBuffer = message.serialize_as().unwrap();

					let array = Uint8Array::new(&buffer);
					assert_eq!(buffer.byte_length(), index.into());
					let mut output = [0; 3];
					array.copy_to(&mut output[..index.into()]);
					assert!(output[..index.into()].iter().all(|value| *value == index));
				}

				flag.signal();
			}
		})
		.spawn(|context| {
			let buffer_1 = ArrayBuffer::new(1);
			let array = Uint8Array::new(&buffer_1);
			array.copy_from(&[1]);

			let buffer_2 = ArrayBuffer::new(2);
			let array = Uint8Array::new(&buffer_2);
			array.copy_from(&[2; 2]);

			let buffer_3 = ArrayBuffer::new(3);
			let array = Uint8Array::new(&buffer_3);
			array.copy_from(&[3; 3]);

			context
				.transfer_messages([buffer_1, buffer_2, buffer_3])
				.unwrap();

			Close::Yes
		});

	flag.await;

	Ok(())
}
