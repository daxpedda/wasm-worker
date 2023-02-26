//! Tests behavior of [`WorkletModule::default()`].

use anyhow::Result;
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::worklet::WorkletModule;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// [`WorkletModule::default()`] and
/// [`WorkletModuleFuture::into_inner()`](wasm_worker::worklet::WorkletModuleFuture::into_inner).
#[wasm_bindgen_test]
async fn test() -> Result<()> {
	let mut future_1 = WorkletModule::default();
	assert!(future_1.into_inner().is_none());

	let mut future_2 = WorkletModule::default();
	assert!(future_2.into_inner().is_none());
	assert!(future_1.into_inner().is_none());

	let mut support = WorkletModule::has_import_support();
	assert!(support.into_inner().is_none());
	assert!(future_2.into_inner().is_none());
	assert!(future_1.into_inner().is_none());

	future_2.await.unwrap();
	assert!(future_1.into_inner().is_some());
	assert!(support.into_inner().is_some());

	let mut future_3 = WorkletModule::default();
	assert!(future_3.into_inner().is_some());

	let mut support = WorkletModule::has_import_support();
	assert!(support.into_inner().is_some());

	Ok(())
}
