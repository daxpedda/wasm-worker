//! Tests behavior of [`Message::has_support()`] with [`Message::ImageBitmap`].

#![cfg(test)]
#![allow(clippy::missing_assert_message)]

use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::message::Message;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// [`Message::has_support`] and
/// [`ImageBitmapSupportFuture::into_inner()`](wasm_worker::message::ImageBitmapSupportFuture::into_inner).
#[wasm_bindgen_test]
async fn test() {
	let message = Message::ImageBitmap(JsValue::UNDEFINED.unchecked_into());

	let mut future_1 = message.has_support().unwrap();
	assert!(future_1.into_inner().is_none());

	let mut future_2 = message.has_support().unwrap();
	assert!(future_2.into_inner().is_none());
	assert!(future_1.into_inner().is_none());

	let mut support = Message::has_image_bitmap_support().unwrap();
	assert!(support.into_inner().is_none());
	assert!(future_2.into_inner().is_none());
	assert!(future_1.into_inner().is_none());

	future_2.await;
	assert!(future_1.into_inner().is_some());
	assert!(support.into_inner().is_some());

	let mut future_3 = message.has_support().unwrap();
	assert!(future_3.into_inner().is_some());

	let mut support = Message::has_image_bitmap_support().unwrap();
	assert!(support.into_inner().is_some());
}
