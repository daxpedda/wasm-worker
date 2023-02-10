//! Test each transferable type to be sent and received successfully. This is
//! important to assert that the support methods of [`Message`] are correct.

mod util;

use std::fmt::Debug;
use std::future::Future;

use futures_util::FutureExt;
use js_sys::{ArrayBuffer, Uint8Array};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::{Close, Message, SupportError, WorkerBuilder};
use web_sys::{
	ImageBitmap, ImageData, MessagePort, OffscreenCanvas, ReadableStream, RtcDataChannel,
	TransformStream, WritableStream,
};
#[cfg(web_sys_unstable_apis)]
use {
	wasm_bindgen::UnwrapThrowExt,
	web_sys::{AudioData, AudioDataCopyToOptions, AudioDataInit, AudioSampleFormat, VideoFrame},
};

use self::util::Flag;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// [`RawMessage::serialize()`](wasm_worker::RawMessage::serialize).
#[wasm_bindgen_test]
async fn serialize() -> Result<(), JsValue> {
	assert!(Message::has_array_buffer_support().is_ok());

	let flag = Flag::new();

	let _worker = WorkerBuilder::new()?
		.message_handler({
			let flag = flag.clone();
			move |_, event| {
				let mut message = event.messages().next().unwrap();

				#[cfg(web_sys_unstable_apis)]
				{
					message = message.serialize_as::<AudioData>().unwrap_err().0;
				}
				message = message.serialize_as::<ImageBitmap>().unwrap_err().0;
				message = message.serialize_as::<MessagePort>().unwrap_err().0;
				message = message.serialize_as::<OffscreenCanvas>().unwrap_err().0;
				message = message.serialize_as::<ReadableStream>().unwrap_err().0;
				message = message.serialize_as::<RtcDataChannel>().unwrap_err().0;
				message = message.serialize_as::<TransformStream>().unwrap_err().0;
				#[cfg(web_sys_unstable_apis)]
				{
					message = message.serialize_as::<VideoFrame>().unwrap_err().0;
				}
				message.serialize_as::<WritableStream>().unwrap_err();

				flag.signal();
			}
		})
		.spawn({
			|context| {
				context.transfer_messages([ArrayBuffer::new(1)]);

				Close::Yes
			}
		});

	flag.await;

	Ok(())
}

/// Tests transfering `T`.
///
/// If `force` is `true` the test will fail if the type is not supported,
/// otherwise the test will be skipped.
async fn test_transfer<T, F>(
	support: Result<(), SupportError>,
	force: bool,
	init: impl Fn() -> F,
	assert_sent: impl 'static + Copy + Fn(&T) + Send,
	assert_received: impl 'static + Copy + Fn(&T) + Send,
) -> Result<(), JsValue>
where
	T: Clone + JsCast + TryFrom<Message>,
	Message: From<T>,
	<T as TryFrom<Message>>::Error: Debug,
	F: Future<Output = T>,
{
	match support {
		Ok(()) => (),
		Err(_) => {
			if force {
				panic!("type unsupported in this browser")
			} else {
				return Ok(());
			}
		}
	}

	let request = Flag::new();
	let response = Flag::new();

	let worker = WorkerBuilder::new()?
		.message_handler({
			let response = response.clone();
			move |_, event| {
				let mut messages = event.messages();
				assert_eq!(messages.len(), 2);

				let value: T = messages.next().unwrap().serialize_as().unwrap();
				assert_received(&value);

				let value = messages.next().unwrap().serialize().unwrap();
				let value: T = value.try_into().unwrap();
				assert_received(&value);

				response.signal();
			}
		})
		.spawn({
			let request = request.clone();
			move |context| {
				context.set_message_handler(move |context, event| {
					let mut messages = event.messages();
					assert_eq!(messages.len(), 2);

					let value_1: T = messages.next().unwrap().serialize_as().unwrap();
					assert_received(&value_1);

					let value_2 = messages.next().unwrap().serialize().unwrap();
					let value_2: T = value_2.try_into().unwrap();
					assert_received(&value_2);

					let old_value = value_1.clone();
					context.transfer_messages([value_1, value_2]);
					assert_sent(&old_value);
				});

				request.signal();

				Close::No
			}
		});

	request.await;

	assert!(Message::from(init().await)
		.has_support()
		.into_inner()
		.unwrap()
		.is_ok());

	let value_1 = init().await;
	let value_2 = init().await;

	let old_value = value_1.clone();
	worker.transfer_messages([value_1, value_2]);
	assert_sent(&old_value);

	response.await;

	worker.terminate();

	Ok(())
}

/// [`ArrayBuffer`].
#[wasm_bindgen_test]
async fn array_buffer() -> Result<(), JsValue> {
	test_transfer(
		Message::has_array_buffer_support(),
		true,
		|| async {
			let buffer = ArrayBuffer::new(1);
			let array = Uint8Array::new(&buffer);
			array.copy_from(&[42]);
			buffer
		},
		|buffer| assert_eq!(buffer.byte_length(), 0),
		|buffer| {
			let array = Uint8Array::new(buffer);
			assert_eq!(array.get_index(0), 42);
		},
	)
	.await
}

/// [`AudioData`].
#[wasm_bindgen_test]
#[cfg(web_sys_unstable_apis)]
async fn audio_data() -> Result<(), JsValue> {
	test_transfer(
		Message::has_audio_data_support(),
		false,
		|| async {
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
		|data| assert_eq!(data.format(), None),
		|data| {
			let size = data.allocation_size(&AudioDataCopyToOptions::new(0));
			assert_eq!(size, 42);
		},
	)
	.await
}

/// [`ImageBitmap`] and
/// [`ImageBitmapSupportFuture::into_inner()`](wasm_worker::ImageBitmapSupportFuture::into_inner).
#[wasm_bindgen_test]
async fn image_bitmap() -> Result<(), JsValue> {
	let mut future = Message::has_image_bitmap_support();
	assert_eq!(future.into_inner(), None);
	assert!(future.await.is_ok());

	assert!(Message::has_image_bitmap_support()
		.into_inner()
		.unwrap()
		.is_ok());

	test_transfer(
		Message::has_image_bitmap_support().into_inner().unwrap(),
		true,
		|| {
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
			assert_eq!(bitmap.width(), 0);
			assert_eq!(bitmap.height(), 0);
		},
		|bitmap| {
			assert_eq!(bitmap.width(), 1);
			assert_eq!(bitmap.height(), 1);
		},
	)
	.await
}
