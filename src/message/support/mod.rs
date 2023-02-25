mod array_buffer;
#[cfg(web_sys_unstable_apis)]
mod audio_data;
mod image_bitmap;
mod message_port;
mod offscreen_canvas;
#[cfg(web_sys_unstable_apis)]
mod readable_stream;
mod rtc_data_channel;
#[allow(clippy::module_inception)]
mod support;
#[cfg(web_sys_unstable_apis)]
mod transform_stream;
#[cfg(web_sys_unstable_apis)]
mod video_frame;
#[cfg(web_sys_unstable_apis)]
mod writable_stream;

use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};

pub use image_bitmap::ImageBitmapSupportFuture;
use js_sys::Array;
pub use support::MessageSupportFuture;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{DomException, Worker};

use super::Message;

impl Message {
	pub fn has_array_buffer_support() -> Result<(), SupportError> {
		array_buffer::support()
	}

	#[cfg(web_sys_unstable_apis)]
	pub fn has_audio_data_support() -> Result<(), SupportError> {
		audio_data::support()
	}

	pub fn has_image_bitmap_support() -> ImageBitmapSupportFuture {
		ImageBitmapSupportFuture::new()
	}

	pub fn has_message_port_support() -> Result<(), SupportError> {
		message_port::support()
	}

	pub fn has_offscreen_canvas_support() -> Result<(), SupportError> {
		offscreen_canvas::support()
	}

	#[cfg(web_sys_unstable_apis)]
	pub fn has_readable_stream_support() -> Result<(), SupportError> {
		readable_stream::support()
	}

	pub fn has_rtc_data_channel_support() -> Result<(), SupportError> {
		rtc_data_channel::support()
	}

	#[cfg(web_sys_unstable_apis)]
	pub fn has_transform_stream_support() -> Result<(), SupportError> {
		transform_stream::support()
	}

	#[cfg(web_sys_unstable_apis)]
	pub fn has_video_frame_support() -> Result<(), SupportError> {
		video_frame::support()
	}

	#[cfg(web_sys_unstable_apis)]
	pub fn has_writable_stream_support() -> Result<(), SupportError> {
		writable_stream::support()
	}
}

fn test_support(data: &JsValue) -> Result<(), SupportError> {
	let worker = Worker::new("data:,").unwrap();
	let result = worker.post_message_with_transfer(data, &Array::of1(data));
	worker.terminate();

	if let Err(error) = result {
		let error: DomException = error.unchecked_into();

		if error.code() == DomException::DATA_CLONE_ERR {
			Err(SupportError::Unsupported)
		} else {
			Err(SupportError::Undetermined)
		}
	} else {
		Ok(())
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupportError {
	Unsupported,
	Undetermined,
}

impl Display for SupportError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Unsupported => write!(f, "type is not transferable"),
			Self::Undetermined => write!(f, "type transfer support couldn't be determined"),
		}
	}
}

impl Error for SupportError {}
