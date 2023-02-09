mod util;

use std::future::Future;

use futures_util::FutureExt;
use js_sys::{ArrayBuffer, Uint8Array};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::{Close, Message, WorkerBuilder};
use web_sys::{ImageBitmap, ImageData};
#[cfg(web_sys_unstable_apis)]
use {
	wasm_bindgen::UnwrapThrowExt,
	web_sys::{AudioData, AudioDataCopyToOptions, AudioDataInit, AudioSampleFormat},
};

use self::util::Flag;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// Tests transfering `T`.
///
/// If `force` is `true` the test will fail if the type is not supported,
/// otherwise the test will be skipped.
async fn test_transfer<T: JsCast + Into<Message>>(
	support: impl Into<Option<bool>>,
	force: bool,
	init: impl Future<Output = T>,
	assert: impl 'static + Copy + Fn(&T) + Send,
) -> Result<(), JsValue> {
	match support.into() {
		None | Some(false) => {
			if force {
				panic!("type unsupported in this browser")
			} else {
				return Ok(());
			}
		}
		Some(true) => (),
	}

	let value = init.await;

	let request = Flag::new();
	let response = Flag::new();

	let worker = WorkerBuilder::new()?
		.message_handler({
			let response = response.clone();
			move |_, event| {
				let value: T = event.messages().next().unwrap().serialize_as().unwrap();

				assert(&value);

				response.signal();
			}
		})
		.spawn({
			let request = request.clone();
			move |context| {
				context.set_message_handler(move |context, event| {
					let value: T = event.messages().next().unwrap().serialize_as().unwrap();

					assert(&value);

					context.transfer_messages([value]);
				});

				request.signal();

				Close::No
			}
		});

	request.await;
	worker.transfer_messages([value]);
	response.await;

	worker.terminate();

	Ok(())
}

#[wasm_bindgen_test]
async fn array_buffer() -> Result<(), JsValue> {
	test_transfer(
		Message::has_array_buffer_support(),
		true,
		async {
			let buffer = ArrayBuffer::new(1);
			let array = Uint8Array::new(&buffer);
			array.copy_from(&[42]);
			buffer
		},
		|buffer| {
			let array = Uint8Array::new(buffer);
			assert_eq!(array.get_index(0), 42);
		},
	)
	.await
}

#[wasm_bindgen_test]
#[cfg(web_sys_unstable_apis)]
async fn audio_data() -> Result<(), JsValue> {
	test_transfer(
		Message::has_audio_data_support(),
		false,
		async {
			let init = AudioDataInit::new(
				&ArrayBuffer::new(42),
				AudioSampleFormat::U8,
				1,
				42,
				3000.,
				0.,
			);
			AudioData::new(&init).unwrap_throw()
		},
		|data| {
			let size = data.allocation_size(&AudioDataCopyToOptions::new(0));
			assert_eq!(size, 42);
		},
	)
	.await
}

#[wasm_bindgen_test]
async fn image_bitmap() -> Result<(), JsValue> {
	test_transfer(
		Message::has_image_bitmap_support().await,
		true,
		{
			let image = ImageData::new_with_sw(1, 1).unwrap();
			let promise = web_sys::window()
				.unwrap()
				.create_image_bitmap_with_image_data(&image)
				.unwrap();

			JsFuture::from(promise)
				.map(Result::unwrap)
				.map(ImageBitmap::unchecked_from_js)
		},
		|bitmap| {
			assert_eq!(bitmap.width(), 1);
			assert_eq!(bitmap.height(), 1);
		},
	)
	.await
}
