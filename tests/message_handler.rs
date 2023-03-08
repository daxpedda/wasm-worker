//! Tests functionality around setting and clearing message handlers in
//! [`WorkerBuilder`], [`Worker`](wasm_worker::worker::Worker) and
//! [`WorkerContext`](wasm_worker::worker::WorkerContext).

mod util;

use std::iter;

use futures_util::future::{self, Either};
use js_sys::{ArrayBuffer, Uint8Array};
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::message::Message;
use wasm_worker::WorkerBuilder;

use self::util::{Flag, SIGNAL_DURATION};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// [`WorkerBuilder::message_handler()`] with
/// [`Worker::clear_message_handler()`](wasm_worker::worker::Worker::clear_message_handler).
#[wasm_bindgen_test]
async fn builder_clear_message_handler() {
	let request = Flag::new();
	let response = Flag::new();

	let worker = WorkerBuilder::new()
		.unwrap()
		.message_handler({
			let response = response.clone();
			move |_, _| response.signal()
		})
		.spawn_async({
			let request = request.clone();
			|context| async move {
				request.await;
				context.transfer_messages(iter::empty::<Message>()).unwrap();

				context.close();
			}
		});

	worker.clear_message_handler();
	request.signal();

	// The message handler will never respond if cleared.
	let result = future::select(response, util::sleep(SIGNAL_DURATION)).await;
	assert!(matches!(result, Either::Right(((), _))));
}

/// [`Worker::set_message_handler()`](wasm_worker::worker::Worker::set_message_handler)
/// with [`Worker::clear_message_handler()`](wasm_worker::worker::Worker::clear_message_handler).
#[wasm_bindgen_test]
async fn handle_clear_message_handler() {
	let request = Flag::new();
	let response = Flag::new();

	let worker = wasm_worker::spawn_async({
		let request = request.clone();
		|context| async move {
			request.await;
			context.transfer_messages(iter::empty::<Message>()).unwrap();

			context.close();
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
}

/// [`WorkerRef::set_message_handler()`](wasm_worker::worker::WorkerRef::set_message_handler) with
/// [`WorkerRef::clear_message_handler()`](wasm_worker::worker::WorkerRef::clear_message_handler).
#[wasm_bindgen_test]
async fn handle_ref_clear_message_handler() {
	let request = Flag::new();
	let response = Flag::new();

	let worker = wasm_worker::spawn_async({
		let request = request.clone();
		|context| async move {
			request.await;
			context.transfer_messages(iter::empty::<Message>()).unwrap();
			context.transfer_messages(iter::empty::<Message>()).unwrap();

			context.close();
		}
	});

	worker.set_message_handler({
		let response = response.clone();
		let mut cleared = false;
		move |worker, _| {
			worker.clear_message_handler();

			if cleared {
				response.signal();
			} else {
				cleared = true;
			}
		}
	});
	request.signal();

	// The message handler will never respond if cleared.
	let result = future::select(response, util::sleep(SIGNAL_DURATION)).await;
	assert!(matches!(result, Either::Right(((), _))));
}

/// [`WorkerContext::set_message_handler()`](wasm_worker::worker::WorkerContext::set_message_handler) with
/// [`WorkerContext::clear_message_handler()`](wasm_worker::worker::WorkerContext::clear_message_handler).
#[wasm_bindgen_test]
async fn context_clear_message_handler() {
	let request = Flag::new();
	let response = Flag::new();

	let worker = wasm_worker::spawn_async({
		let request = request.clone();
		let response = response.clone();
		|context| async move {
			context.set_message_handler(move |_, _| response.signal());
			context.clear_message_handler();
			request.signal();
		}
	});

	request.await;
	worker.transfer_messages(iter::empty::<Message>()).unwrap();

	// The message handler will never respond if cleared.
	let result = future::select(response, util::sleep(SIGNAL_DURATION)).await;
	assert!(matches!(result, Either::Right(((), _))));

	worker.terminate();
}

/// [`WorkerBuilder::message_handler()`] with
/// [`Worker::builder_has_message_handler()`](wasm_worker::worker::Worker::builder_has_message_handler).
#[wasm_bindgen_test]
fn builder_has_message_handler() {
	let worker = WorkerBuilder::new()
		.unwrap()
		.message_handler(|_, _| ())
		.spawn(|context| context.close());
	assert!(worker.has_message_handler());
	worker.clear_message_handler();
	assert!(!worker.has_message_handler());
}

/// [`WorkerBuilder::worker_message_handler()`] with
/// [`WorkerContext::has_message_handler()`](wasm_worker::worker::WorkerContext::has_message_handler).
#[wasm_bindgen_test]
async fn builder_worker_has_message_handler() {
	let flag = Flag::new();

	WorkerBuilder::new()
		.unwrap()
		.worker_message_handler(|_, _| ())
		.spawn({
			let flag = flag.clone();
			move |context| {
				assert!(context.has_message_handler());
				context.clear_message_handler();
				assert!(!context.has_message_handler());
				context.set_message_handler(|_, _| ());
				assert!(context.has_message_handler());

				// Flag will never signal if `assert!`s panic.
				flag.signal();

				context.close();
			}
		});

	flag.await;
}

/// [`Worker::set_message_handler()`](wasm_worker::worker::Worker::set_message_handler)
/// with [`Worker::has_message_handler()`](wasm_worker::worker::Worker::has_message_handler).
#[wasm_bindgen_test]
fn handle_has_message_handler() {
	let worker = wasm_worker::spawn(|context| context.close());

	assert!(!worker.has_message_handler());
	worker.set_message_handler(|_, _| ());
	assert!(worker.has_message_handler());
	worker.clear_message_handler();
	assert!(!worker.has_message_handler());
}

/// [`WorkerRef::set_message_handler()`](wasm_worker::worker::WorkerRef::set_message_handler) with
/// [`WorkerRef::has_message_handler()`](wasm_worker::worker::WorkerRef::has_message_handler).
#[wasm_bindgen_test]
async fn handle_ref_has_message_handler() {
	let flag = Flag::new();

	let _worker = WorkerBuilder::new()
		.unwrap()
		.message_handler({
			let flag = flag.clone();
			move |worker, _| {
				assert!(worker.has_message_handler());
				worker.clear_message_handler();
				assert!(!worker.has_message_handler());

				// Flag will never signal if `assert!`s panic.
				flag.signal();
			}
		})
		.spawn(|context| {
			context.transfer_messages(iter::empty::<Message>()).unwrap();
			context.close();
		});

	flag.await;
}

/// [`WorkerContext::has_message_handler()`](wasm_worker::worker::WorkerContext::has_message_handler).
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

			context.close();
		}
	});

	flag.await;
}

/// [`WorkerBuilder::message_handler()`].
#[wasm_bindgen_test]
async fn builder_message_handler() {
	let flag = Flag::new();

	let _worker = WorkerBuilder::new()
		.unwrap()
		.message_handler({
			let flag = flag.clone();
			move |_, _| flag.signal()
		})
		.spawn({
			|context| {
				context.transfer_messages(iter::empty::<Message>()).unwrap();

				context.close();
			}
		});

	flag.await;
}

/// [`WorkerBuilder::worker_message_handler()`].
#[wasm_bindgen_test]
async fn builder_worker_message_handler() {
	let flag = Flag::new();

	let worker = WorkerBuilder::new()
		.unwrap()
		.worker_message_handler({
			let flag = flag.clone();
			move |_, _| flag.signal()
		})
		.spawn(|_| ());

	worker.transfer_messages(iter::empty::<Message>()).unwrap();
	flag.await;

	worker.terminate();
}

/// [`Worker::set_message_handler()`](wasm_worker::worker::Worker::set_message_handler).
#[wasm_bindgen_test]
async fn handle_message_handler() {
	let request = Flag::new();
	let response = Flag::new();

	let worker = wasm_worker::spawn_async({
		let request = request.clone();
		|context| async move {
			request.await;
			context.transfer_messages(iter::empty::<Message>()).unwrap();

			context.close();
		}
	});

	worker.set_message_handler({
		let response = response.clone();
		move |_, _| response.signal()
	});
	request.signal();

	response.await;
}

/// [`WorkerRef::set_message_handler()`](wasm_worker::worker::WorkerRef::set_message_handler).
#[wasm_bindgen_test]
async fn handle_ref_message_handler() {
	let flag = Flag::new();

	let _worker = WorkerBuilder::new()
		.unwrap()
		.message_handler({
			let flag = flag.clone();
			move |worker, _| {
				worker.set_message_handler({
					let flag = flag.clone();
					move |_, _| flag.signal()
				});
			}
		})
		.spawn({
			|context| {
				context.transfer_messages(iter::empty::<Message>()).unwrap();
				context.transfer_messages(iter::empty::<Message>()).unwrap();

				context.close();
			}
		});

	flag.await;
}

/// [`WorkerContext::set_message_handler()`](wasm_worker::worker::WorkerContext::set_message_handler).
#[wasm_bindgen_test]
async fn context_message_handler() {
	let request = Flag::new();
	let response = Flag::new();

	let worker = wasm_worker::spawn_async({
		let request = request.clone();
		let response = response.clone();
		|context| async move {
			context.set_message_handler(move |_, _| response.signal());
			request.signal();
		}
	});

	request.await;
	worker.transfer_messages(iter::empty::<Message>()).unwrap();

	response.await;

	worker.terminate();
}

/// [`WorkerBuilder::message_handler_async()`].
#[wasm_bindgen_test]
async fn builder_message_handler_async() {
	let flag = Flag::new();

	let _worker = WorkerBuilder::new()
		.unwrap()
		.message_handler_async({
			let flag = flag.clone();
			move |_, _| {
				let flag = flag.clone();
				async move { flag.signal() }
			}
		})
		.spawn({
			|context| {
				context.transfer_messages(iter::empty::<Message>()).unwrap();

				context.close();
			}
		});

	flag.await;
}

/// [`WorkerBuilder::worker_message_handler_async()`].
#[wasm_bindgen_test]
async fn builder_worker_message_handler_async() {
	let flag = Flag::new();

	let worker = WorkerBuilder::new()
		.unwrap()
		.worker_message_handler_async({
			let flag = flag.clone();
			move |_, _| {
				let flag = flag.clone();
				async move { flag.signal() }
			}
		})
		.spawn(|_| ());

	worker.transfer_messages(iter::empty::<Message>()).unwrap();
	flag.await;

	worker.terminate();
}

/// [`Worker::set_message_handler_async()`](wasm_worker::worker::Worker::set_message_handler_async).
#[wasm_bindgen_test]
async fn handle_message_handler_async() {
	let request = Flag::new();
	let response = Flag::new();

	let worker = wasm_worker::spawn_async({
		let request = request.clone();
		|context| async move {
			request.await;
			context.transfer_messages(iter::empty::<Message>()).unwrap();

			context.close();
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

/// [`WorkerRef::set_message_handler()`](wasm_worker::worker::WorkerRef::set_message_handler).
#[wasm_bindgen_test]
async fn handle_ref_message_handler_async() {
	let flag = Flag::new();

	let _worker = WorkerBuilder::new()
		.unwrap()
		.message_handler({
			let flag = flag.clone();
			move |worker, _| {
				worker.set_message_handler_async({
					let flag = flag.clone();
					move |_, _| {
						let flag = flag.clone();
						async move { flag.signal() }
					}
				});
			}
		})
		.spawn({
			|context| {
				context.transfer_messages(iter::empty::<Message>()).unwrap();
				context.transfer_messages(iter::empty::<Message>()).unwrap();

				context.close();
			}
		});

	flag.await;
}

/// [`WorkerContext::set_message_handler_async()`](wasm_worker::worker::WorkerContext::set_message_handler_async).
#[wasm_bindgen_test]
async fn context_message_handler_async() {
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
		}
	});

	request.await;
	worker.transfer_messages(iter::empty::<Message>()).unwrap();

	response.await;

	worker.terminate();
}

/// [`WorkerBuilder::message_handler()`] when
/// [`Worker`](wasm_worker::worker::Worker) is dropped.
#[wasm_bindgen_test]
async fn builder_drop_message_handler() {
	let request = Flag::new();
	let response = Flag::new();

	WorkerBuilder::new()
		.unwrap()
		.message_handler({
			let response = response.clone();
			move |_, _| response.signal()
		})
		.spawn_async({
			let request = request.clone();
			|context| async move {
				request.await;
				context.transfer_messages(iter::empty::<Message>()).unwrap();

				context.close();
			}
		});

	request.signal();

	// The message handler will never respond if dropped.
	let result = future::select(response, util::sleep(SIGNAL_DURATION)).await;
	assert!(matches!(result, Either::Right(((), _))));
}

/// [`Worker::set_message_handler()`](wasm_worker::worker::Worker::set_message_handler)
/// when [`Worker`](wasm_worker::worker::Worker) is dropped.
#[wasm_bindgen_test]
async fn handle_drop_message_handler() {
	let request = Flag::new();
	let response = Flag::new();

	let worker = wasm_worker::spawn_async({
		let request = request.clone();
		|context| async move {
			request.await;
			context.transfer_messages(iter::empty::<Message>()).unwrap();

			context.close();
		}
	});

	worker.set_message_handler({
		let response = response.clone();
		move |_, _| response.signal()
	});
	drop(worker);
	request.signal();

	// The message handler will never respond if dropped.
	let result = future::select(response, util::sleep(SIGNAL_DURATION)).await;
	assert!(matches!(result, Either::Right(((), _))));
}

/// No messages in
/// [`Worker::transfer_messages()`](wasm_worker::worker::Worker::transfer_messages).
#[wasm_bindgen_test]
async fn handle_no_message() {
	let flag = Flag::new();

	let worker = WorkerBuilder::new()
		.unwrap()
		.worker_message_handler({
			let flag = flag.clone();
			move |_, event| {
				let mut messages = event.messages().unwrap().into_iter();
				assert_eq!(messages.len(), 0);
				assert!(matches!(messages.next(), None));

				flag.signal();
			}
		})
		.spawn(|_| ());

	worker.transfer_messages(iter::empty::<Message>()).unwrap();

	flag.await;

	worker.terminate();
}

/// No messages in
/// [`WorkerRef::transfer_messages()`](wasm_worker::worker::WorkerRef::transfer_messages).
#[wasm_bindgen_test]
async fn handle_ref_no_message() {
	let flag = Flag::new();

	let worker = WorkerBuilder::new()
		.unwrap()
		.message_handler(move |worker, _| {
			worker.transfer_messages(iter::empty::<Message>()).unwrap();
		})
		.worker_message_handler({
			let flag = flag.clone();
			move |_, event| {
				let mut messages = event.messages().unwrap().into_iter();
				assert_eq!(messages.len(), 0);
				assert!(matches!(messages.next(), None));

				flag.signal();
			}
		})
		.spawn(|context| context.transfer_messages(iter::empty::<Message>()).unwrap());

	flag.await;

	worker.terminate();
}

/// No messages in
/// [`WorkerContext::transfer_messages()`](wasm_worker::worker::WorkerContext::transfer_messages).
#[wasm_bindgen_test]
async fn context_no_message() {
	let flag = Flag::new();

	let _worker = WorkerBuilder::new()
		.unwrap()
		.message_handler({
			let flag = flag.clone();
			move |_, event| {
				let mut messages = event.messages().unwrap().into_iter();
				assert_eq!(messages.len(), 0);
				assert!(matches!(messages.next(), None));

				flag.signal();
			}
		})
		.spawn(|context| {
			context.transfer_messages(iter::empty::<Message>()).unwrap();

			context.close();
		});

	flag.await;
}

/// Multiple messages in
/// [`Worker::transfer_messages()`](wasm_worker::worker::Worker::transfer_messages).
#[wasm_bindgen_test]
async fn handle_multi_message() {
	assert!(Message::has_array_buffer_support().is_ok());

	let flag = Flag::new();

	let worker = WorkerBuilder::new()
		.unwrap()
		.worker_message_handler({
			let flag = flag.clone();
			move |_, event| {
				let messages = event.messages().unwrap().into_iter();
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
		.spawn(|_| ());

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

	flag.await;

	worker.terminate();
}

/// Multiple messages in
/// [`WorkerRef::transfer_messages()`](wasm_worker::worker::WorkerRef::transfer_messages).
#[wasm_bindgen_test]
async fn handle_ref_multi_message() {
	assert!(Message::has_array_buffer_support().is_ok());

	let flag = Flag::new();

	let worker = WorkerBuilder::new()
		.unwrap()
		.message_handler(move |worker, _| {
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
		})
		.worker_message_handler({
			let flag = flag.clone();
			move |_, event| {
				let messages = event.messages().unwrap().into_iter();
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
		.spawn(|context| context.transfer_messages(iter::empty::<Message>()).unwrap());

	flag.await;

	worker.terminate();
}

/// Multiple messages in
/// [`WorkerContext::transfer_messages()`](wasm_worker::worker::WorkerContext::transfer_messages).
#[wasm_bindgen_test]
async fn context_multi_message() {
	assert!(Message::has_array_buffer_support().is_ok());

	let flag = Flag::new();

	let _worker = WorkerBuilder::new()
		.unwrap()
		.message_handler({
			let flag = flag.clone();
			move |_, event| {
				let messages = event.messages().unwrap().into_iter();

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

			context.close();
		});

	flag.await;
}
