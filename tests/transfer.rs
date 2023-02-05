#![allow(unreachable_pub)]

mod util;

use js_sys::{ArrayBuffer, Uint8Array};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::{Close, Message, WorkerBuilder};

use self::util::Flag;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn array_buffer() -> Result<(), JsValue> {
	assert!(Message::has_array_buffer_support());

	let buffer = ArrayBuffer::new(1);
	let array = Uint8Array::new(&buffer);
	array.copy_from(&[42]);

	let flag_start = Flag::new();
	let flag_finish = Flag::new();

	let worker = WorkerBuilder::new()?
		.set_message_handler({
			let flag_finish = flag_finish.clone();
			move |event| {
				let mut messages = event.messages().unwrap();

				if let Ok(Message::ArrayBuffer(buffer)) =
					messages.next().unwrap().serialize_as::<ArrayBuffer>()
				{
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
			|context| async move {
				context.set_message_handler(move |context, event| {
					let mut messages = event.messages().unwrap();

					if let Ok(Message::ArrayBuffer(buffer)) =
						messages.next().unwrap().serialize_as::<ArrayBuffer>()
					{
						let array = Uint8Array::new(&buffer);
						assert_eq!(array.get_index(0), 42);

						context.transfer_messages([buffer.into()]);
					} else {
						panic!()
					}
				});

				flag_start.signal();

				Close::No
			}
		});

	flag_start.await;
	worker.transfer_message([buffer.into()]);
	flag_finish.await;

	worker.terminate();

	Ok(())
}
