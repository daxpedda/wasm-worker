use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::Deref;

use js_sys::{Array, ArrayBuffer};
use once_cell::sync::{Lazy, OnceCell};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use wasm_bindgen_futures::JsFuture;
#[cfg(web_sys_unstable_apis)]
use web_sys::{AudioData, AudioDataInit, AudioSampleFormat, VideoFrame};
use web_sys::{
	ImageBitmap, ImageData, MessagePort, OffscreenCanvas, ReadableStream, RtcDataChannel,
	TransformStream, Window, Worker, WorkerGlobalScope, WritableStream,
};

#[derive(Debug)]
pub enum Message {
	ArrayBuffer(ArrayBuffer),
	#[cfg(web_sys_unstable_apis)]
	AudioData(AudioData),
	ImageBitmap(ImageBitmap),
	MessagePort(MessagePort),
	OffscreenCanvas(OffscreenCanvas),
	ReadableStream(ReadableStream),
	RtcDataChannel(RtcDataChannel),
	TransformStream(TransformStream),
	#[cfg(web_sys_unstable_apis)]
	VideoFrame(VideoFrame),
	WritableStream(WritableStream),
}

impl From<ArrayBuffer> for Message {
	fn from(value: ArrayBuffer) -> Self {
		Self::ArrayBuffer(value)
	}
}

#[cfg(web_sys_unstable_apis)]
impl From<AudioData> for Message {
	fn from(value: AudioData) -> Self {
		Self::AudioData(value)
	}
}

impl From<ImageBitmap> for Message {
	fn from(value: ImageBitmap) -> Self {
		Self::ImageBitmap(value)
	}
}

impl Message {
	pub(crate) fn from_js_value(data: JsValue) -> Result<Self, MessageError> {
		if data.is_instance_of::<ArrayBuffer>() {
			return Ok(Self::ArrayBuffer(data.unchecked_into()));
		}

		#[cfg(web_sys_unstable_apis)]
		if data.is_instance_of::<AudioData>() {
			return Ok(Self::AudioData(data.unchecked_into()));
		}

		if data.is_instance_of::<ImageBitmap>() {
			return Ok(Self::ImageBitmap(data.unchecked_into()));
		}

		if data.is_instance_of::<MessagePort>() {
			return Ok(Self::MessagePort(data.unchecked_into()));
		}

		if data.is_instance_of::<OffscreenCanvas>() {
			return Ok(Self::OffscreenCanvas(data.unchecked_into()));
		}

		if data.is_instance_of::<ReadableStream>() {
			return Ok(Self::ReadableStream(data.unchecked_into()));
		}

		if data.is_instance_of::<RtcDataChannel>() {
			return Ok(Self::RtcDataChannel(data.unchecked_into()));
		}

		if data.is_instance_of::<TransformStream>() {
			return Ok(Self::TransformStream(data.unchecked_into()));
		}

		#[cfg(web_sys_unstable_apis)]
		if data.is_instance_of::<VideoFrame>() {
			return Ok(Self::VideoFrame(data.unchecked_into()));
		}

		if data.is_instance_of::<WritableStream>() {
			return Ok(Self::WritableStream(data.unchecked_into()));
		}

		Err(MessageError(data))
	}

	pub(crate) fn into_js_value(self) -> JsValue {
		match self {
			Self::ArrayBuffer(value) => value.into(),
			#[cfg(web_sys_unstable_apis)]
			Self::AudioData(value) => value.into(),
			Self::ImageBitmap(value) => value.into(),
			Self::MessagePort(value) => value.into(),
			Self::OffscreenCanvas(value) => value.into(),
			Self::ReadableStream(value) => value.into(),
			Self::RtcDataChannel(value) => value.into(),
			Self::TransformStream(value) => value.into(),
			#[cfg(web_sys_unstable_apis)]
			Self::VideoFrame(value) => value.into(),
			Self::WritableStream(value) => value.into(),
		}
	}

	#[must_use]
	pub fn has_array_buffer_support() -> bool {
		static SUPPORT: Lazy<bool> = Lazy::new(|| {
			let buffer = ArrayBuffer::new(1);

			let worker = Worker::new("data:,").unwrap_throw();
			worker
				.post_message_with_transfer(&buffer, &Array::of1(&buffer))
				.unwrap_throw();
			worker.terminate();

			buffer.byte_length() == 0
		});

		*SUPPORT
	}

	#[must_use]
	#[cfg(web_sys_unstable_apis)]
	pub fn has_audio_data_support() -> bool {
		static SUPPORT: Lazy<bool> = Lazy::new(|| {
			#[wasm_bindgen]
			extern "C" {
				type AudioDataGlobal;

				#[wasm_bindgen(method, getter, js_name = AudioData)]
				fn audio_data(this: &AudioDataGlobal) -> JsValue;
			}

			let global: AudioDataGlobal = js_sys::global().unchecked_into();

			if global.audio_data().is_undefined() {
				return false;
			}

			let init =
				AudioDataInit::new(&ArrayBuffer::new(1), AudioSampleFormat::U8, 1, 1, 3000., 0.);
			let data = AudioData::new(&init).unwrap_throw();

			let worker = Worker::new("data:,").unwrap_throw();
			worker
				.post_message_with_transfer(&data, &Array::of1(&data))
				.unwrap_throw();
			worker.terminate();

			data.format().is_none()
		});

		*SUPPORT
	}

	pub async fn has_image_bitmap_support() -> Option<bool> {
		enum Global {
			Window(Window),
			Worker(WorkerGlobalScope),
		}

		static SUPPORT: OnceCell<bool> = OnceCell::new();

		thread_local! {
			static GLOBAL: Lazy<Option<Global>> = Lazy::new(|| {
				#[wasm_bindgen]
				extern "C" {
					type ImageBitmapGlobal;

					#[wasm_bindgen(method, getter, js_name = Window)]
					fn window(this: &ImageBitmapGlobal) -> JsValue;

					#[wasm_bindgen(method, getter, js_name = WorkerGlobalScope)]
					fn worker(this: &ImageBitmapGlobal) -> JsValue;
				}

				let global: ImageBitmapGlobal = js_sys::global().unchecked_into();

				if !global.window().is_undefined() {
					Some(Global::Window(global.unchecked_into()))
				} else if !global.worker().is_undefined() {
					Some(Global::Worker(global.unchecked_into()))
				} else {
					None
				}
			});
		}

		if let Some(support) = SUPPORT.get() {
			return Some(*support);
		}

		let promise = GLOBAL.with(|global| {
			if let Some(global) = global.deref() {
				let image = ImageData::new_with_sw(1, 1).unwrap();

				match global {
					Global::Window(window) => window.create_image_bitmap_with_image_data(&image),
					Global::Worker(worker) => worker.create_image_bitmap_with_image_data(&image),
				}
				.ok()
			} else {
				None
			}
		})?;

		let bitmap: ImageBitmap = JsFuture::from(promise).await.unwrap().unchecked_into();

		let worker = Worker::new("data:,").unwrap_throw();
		worker
			.post_message_with_transfer(&bitmap, &Array::of1(&bitmap))
			.unwrap_throw();
		worker.terminate();

		Some(bitmap.width() == 0)
	}
}

#[derive(Debug)]
pub struct RawMessage(pub(crate) JsValue);

impl RawMessage {
	#[must_use]
	#[allow(clippy::missing_const_for_fn)]
	pub fn into_raw(self) -> JsValue {
		self.0
	}

	pub fn serialize(self) -> Result<Message, MessageError> {
		Message::from_js_value(self.0)
	}

	pub fn serialize_as<T: JsCast + Into<Message>>(self) -> Result<T, MessageError> {
		if self.0.is_instance_of::<T>() {
			Ok(self.0.unchecked_into::<T>())
		} else {
			Err(MessageError(self.0))
		}
	}
}

#[derive(Debug)]
pub struct MessageError(JsValue);

impl Display for MessageError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "unexpected message: {:?}", self.0)
	}
}

impl Error for MessageError {}

impl From<MessageError> for JsValue {
	fn from(value: MessageError) -> Self {
		value.to_string().into()
	}
}

impl MessageError {
	#[must_use]
	#[allow(clippy::missing_const_for_fn)]
	pub fn into_raw(self) -> JsValue {
		self.0
	}
}
