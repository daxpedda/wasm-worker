//! Tests behavior of
//! [`WorkletUrlFuture::into_inner()`](wasm_worker::worklet::WorkletUrlFuture::into_inner).

use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::worklet::WorkletUrl;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// [`WorkletUrlFuture::into_inner()`](wasm_worker::worklet::WorkletUrlFuture::into_inner)
/// and [`WorkletUrl::default()`].
#[wasm_bindgen_test]
async fn module() {
	if !matches!(
		wasm_bindgen::shim_format().unwrap(),
		wasm_bindgen::ShimFormat::EsModule
	) {
		return;
	}

	let mut future_1 = WorkletUrl::has_import_support();
	assert!(future_1.into_inner().is_none());

	let mut future_2 = WorkletUrl::has_import_support();
	assert!(future_2.into_inner().is_none());
	assert!(future_1.into_inner().is_none());

	let mut module = WorkletUrl::default();
	assert!(module.into_inner().is_none());
	assert!(future_2.into_inner().is_none());
	assert!(future_1.into_inner().is_none());

	future_2.await;
	assert!(future_1.into_inner().is_some());
	assert!(module.into_inner().is_some());

	let mut future_3 = WorkletUrl::has_import_support();
	assert!(future_3.into_inner().is_some());

	let mut support = WorkletUrl::default();
	assert!(support.into_inner().is_some());
}

/// [`WorkletUrlFuture::into_inner()`](wasm_worker::worklet::WorkletUrlFuture::into_inner)
/// and [`WorkletUrl::default()`].
#[wasm_bindgen_test]
async fn classic() {
	if !matches!(
		wasm_bindgen::shim_format().unwrap(),
		wasm_bindgen::ShimFormat::NoModules { .. }
	) {
		return;
	}

	let mut future_1 = WorkletUrl::has_import_support();
	assert!(future_1.into_inner().is_none());

	let mut future_2 = WorkletUrl::has_import_support();
	assert!(future_2.into_inner().is_none());
	assert!(future_1.into_inner().is_none());

	let mut module_1 = WorkletUrl::default();
	assert!(module_1.into_inner().is_none());
	assert!(future_2.into_inner().is_none());
	assert!(future_1.into_inner().is_none());

	future_2.await;
	assert!(future_1.into_inner().is_some());
	assert!(module_1.into_inner().is_none());

	let mut future_3 = WorkletUrl::has_import_support();
	assert!(future_3.into_inner().is_some());

	let mut module_2 = WorkletUrl::default();
	assert!(module_2.into_inner().is_none());

	module_1.await.unwrap();
	assert!(module_2.into_inner().is_some());
}
