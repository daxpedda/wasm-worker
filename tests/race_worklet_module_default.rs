//! Tests behavior of [`WorkletUrl::default()`].

use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::worklet::WorkletUrl;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// [`WorkletUrl::default()`] and
/// [`WorkletUrlFuture::into_inner()`](wasm_worker::worklet::WorkletUrlFuture::into_inner).
#[wasm_bindgen_test]
async fn module() {
	if !matches!(
		wasm_bindgen::shim_format().unwrap(),
		wasm_bindgen::ShimFormat::EsModule
	) {
		return;
	}

	let mut future_1 = WorkletUrl::default();
	assert!(future_1.into_inner().is_none());

	let mut future_2 = WorkletUrl::default();
	assert!(future_2.into_inner().is_none());
	assert!(future_1.into_inner().is_none());

	let mut support = WorkletUrl::has_import_support();
	assert!(support.into_inner().is_none());
	assert!(future_2.into_inner().is_none());
	assert!(future_1.into_inner().is_none());

	future_2.await.unwrap();
	assert!(future_1.into_inner().is_some());
	assert!(support.into_inner().is_some());

	let mut future_3 = WorkletUrl::default();
	assert!(future_3.into_inner().is_some());

	let mut support = WorkletUrl::has_import_support();
	assert!(support.into_inner().is_some());
}

/// [`WorkletUrl::default()`] and
/// [`WorkletUrlFuture::into_inner()`](wasm_worker::worklet::WorkletUrlFuture::into_inner).
#[wasm_bindgen_test]
async fn classic() {
	if !matches!(
		wasm_bindgen::shim_format().unwrap(),
		wasm_bindgen::ShimFormat::NoModules { .. }
	) {
		return;
	}

	let mut future_1 = WorkletUrl::default();
	assert!(future_1.into_inner().is_none());

	let mut future_2 = WorkletUrl::default();
	assert!(future_2.into_inner().is_none());
	assert!(future_1.into_inner().is_none());

	let mut support_1 = WorkletUrl::has_import_support();
	assert!(support_1.into_inner().is_none());
	assert!(future_2.into_inner().is_none());
	assert!(future_1.into_inner().is_none());

	future_2.await.unwrap();
	assert!(future_1.into_inner().is_some());
	assert!(support_1.into_inner().is_none());

	let mut future_3 = WorkletUrl::default();
	assert!(future_3.into_inner().is_some());

	let mut support_2 = WorkletUrl::has_import_support();
	assert!(support_2.into_inner().is_none());

	support_1.await;
	assert!(support_2.into_inner().is_some());
}
