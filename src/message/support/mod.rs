mod array_buffer;
#[cfg(web_sys_unstable_apis)]
mod audio_data;
mod image_bitmap;
mod message_port;
mod offscreen_canvas;
mod readable_stream;
mod rtc_data_channel;
#[allow(clippy::module_inception)]
mod support;
mod transform_stream;
#[cfg(web_sys_unstable_apis)]
mod video_frame;
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
	pub fn has_array_buffer_support() -> Result<bool, MessageSupportError> {
		array_buffer::support()
	}

	#[cfg(web_sys_unstable_apis)]
	pub fn has_audio_data_support() -> Result<bool, MessageSupportError> {
		audio_data::support()
	}

	pub fn has_image_bitmap_support() -> Result<ImageBitmapSupportFuture, MessageSupportError> {
		ImageBitmapSupportFuture::new()
	}

	pub fn has_message_port_support() -> Result<bool, MessageSupportError> {
		message_port::support()
	}

	pub fn has_offscreen_canvas_support() -> Result<bool, MessageSupportError> {
		offscreen_canvas::support()
	}

	pub fn has_readable_stream_support() -> Result<bool, MessageSupportError> {
		readable_stream::support()
	}

	pub fn has_rtc_data_channel_support() -> Result<bool, MessageSupportError> {
		rtc_data_channel::support()
	}

	pub fn has_transform_stream_support() -> Result<bool, MessageSupportError> {
		transform_stream::support()
	}

	#[cfg(web_sys_unstable_apis)]
	pub fn has_video_frame_support() -> Result<bool, MessageSupportError> {
		video_frame::support()
	}

	pub fn has_writable_stream_support() -> Result<bool, MessageSupportError> {
		writable_stream::support()
	}
}

fn test_support(data: &JsValue) -> bool {
	let worker = Worker::new("data:,").unwrap();
	let result = worker.post_message_with_transfer(data, &Array::of1(data));
	worker.terminate();

	if let Err(error) = result {
		debug_assert_eq!(
			DomException::unchecked_from_js(error).code(),
			DomException::DATA_CLONE_ERR
		);

		false
	} else {
		true
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MessageSupportError;

impl Display for MessageSupportError {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "context can't be used to determine support")
	}
}

impl Error for MessageSupportError {}
