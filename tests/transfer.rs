#![allow(unreachable_pub)]

mod util;

use js_sys::{ArrayBuffer, Uint8Array};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::{Close, Message, WorkerBuilder};

use self::util::Flag;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn array_buffer_transfer() -> Result<(), JsValue> {
	let buffer = ArrayBuffer::new(1);
	let array = Uint8Array::new(&buffer);
	array.copy_from(&[42]);

	let flag_start = Flag::new();
	let flag_sent = Flag::new();
	let flag_finish = Flag::new();

	let worker = WorkerBuilder::new()?
		.set_message_handler({
			let flag_finish = flag_finish.clone();
			move |event| {
				if let Some(Message::ArrayBuffer(buffer)) = event.message() {
					let array = Uint8Array::new(&buffer);
					assert_eq!(array.get_index(0), 42);
				} else {
					panic!()
				}

				flag_finish.signal();
			}
		})
		.spawn({
			let flag_start = flag_start.clone();
			let flag_sent = flag_sent.clone();
			|context| async move {
				context.set_message_handler(move |context, event| {
					if let Some(Message::ArrayBuffer(buffer)) = event.message() {
						let array = Uint8Array::new(&buffer);
						assert_eq!(array.get_index(0), 42);

						assert!(context.transfer_message(buffer.into()).is_ok());
					} else {
						panic!()
					}

					flag_sent.signal();
				});

				flag_start.signal();

				Close::No
			}
		});

	flag_start.await;

	assert!(worker
		.transfer_message(Message::ArrayBuffer(buffer))
		.is_ok());

	flag_sent.await;
	flag_finish.await;

	worker.terminate();

	Ok(())
}
