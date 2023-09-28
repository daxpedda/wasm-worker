//! Tests behavior of
//! [`ImageBitmapSupportFuture::into_inner()`](wasm_worker::message::ImageBitmapSupportFuture::into_inner).

#![cfg(test)]
#![allow(clippy::missing_assert_message)]

use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::message::Message;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// [`ImageBitmapSupportFuture::into_inner()`](wasm_worker::message::ImageBitmapSupportFuture::into_inner)
/// and [`Message::has_support`].
#[wasm_bindgen_test]
async fn test() {
	let mut future_1 = Message::has_image_bitmap_support().unwrap();
	assert!(future_1.into_inner().is_none());

	let mut future_2 = Message::has_image_bitmap_support().unwrap();
	assert!(future_2.into_inner().is_none());
	assert!(future_1.into_inner().is_none());

	let message = Message::ImageBitmap(JsValue::UNDEFINED.unchecked_into());
	let mut support = message.has_support().unwrap();
	assert!(support.into_inner().is_none());
	assert!(future_2.into_inner().is_none());
	assert!(future_1.into_inner().is_none());

	future_2.await;
	assert!(future_1.into_inner().is_some());
	assert!(support.into_inner().is_some());

	let mut future_3 = Message::has_image_bitmap_support().unwrap();
	assert!(future_3.into_inner().is_some());

	let mut support = message.has_support().unwrap();
	assert!(support.into_inner().is_some());
}
