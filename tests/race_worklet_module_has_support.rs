//! Tests behavior of
//! [`WorkletModuleFuture::into_inner()`](wasm_worker::worklet::WorkletModuleFuture::into_inner).

use anyhow::Result;
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::worklet::WorkletModule;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// [`WorkletModuleFuture::into_inner()`](wasm_worker::worklet::WorkletModuleFuture::into_inner)
/// and [`WorkletModule::default()`].
#[wasm_bindgen_test]
async fn test() -> Result<()> {
	let mut future_1 = WorkletModule::has_import_support();
	assert!(future_1.into_inner().is_none());

	let mut future_2 = WorkletModule::has_import_support();
	assert!(future_2.into_inner().is_none());
	assert!(future_1.into_inner().is_none());

	let mut module = WorkletModule::default();
	assert!(module.into_inner().is_none());
	assert!(future_2.into_inner().is_none());
	assert!(future_1.into_inner().is_none());

	future_2.await;
	assert!(future_1.into_inner().is_some());
	assert!(module.into_inner().is_some());

	let mut future_3 = WorkletModule::has_import_support();
	assert!(future_3.into_inner().is_some());

	let mut support = WorkletModule::default();
	assert!(support.into_inner().is_some());

	Ok(())
}
