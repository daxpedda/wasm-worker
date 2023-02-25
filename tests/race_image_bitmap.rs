//! Tests behavior of
//! [`ImageBitmapSupportFuture::into_inner()`](wasm_worker::ImageBitmapSupportFuture::into_inner).

use anyhow::Result;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::message::Message;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// [`ImageBitmapSupportFuture::into_inner()`](wasm_worker::ImageBitmapSupportFuture::into_inner)
/// and [`Message::has_support`].
#[wasm_bindgen_test]
async fn test() -> Result<()> {
	let mut future_1 = Message::has_image_bitmap_support();
	assert!(future_1.into_inner().is_none());

	let mut future_2 = Message::has_image_bitmap_support();
	assert!(future_2.into_inner().is_none());
	assert!(future_1.into_inner().is_none());

	let message = Message::ImageBitmap(JsValue::UNDEFINED.unchecked_into());
	let mut support = message.has_support();
	assert!(support.into_inner().is_none());
	assert!(future_2.into_inner().is_none());
	assert!(future_1.into_inner().is_none());

	future_2.await?;
	assert!(future_1.into_inner().is_some());
	assert!(support.into_inner().is_some());

	let mut future_3 = Message::has_image_bitmap_support();
	assert!(future_3.into_inner().is_some());

	let mut support = message.has_support();
	assert!(support.into_inner().is_some());

	Ok(())
}
