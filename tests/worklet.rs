//! Tests audio worklet functionality.

mod util;

use futures_util::future::{self, Either};
use util::Flag;
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::common::ShimFormat;
use wasm_worker::worklet::{self, WorkletContext, WorkletInitError, WorkletUrl};
use wasm_worker::{WorkletBuilder, WorkletExt};
use web_sys::OfflineAudioContext;

use crate::util::SIGNAL_DURATION;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// [`WorkletExt::add_wasm()`].
#[wasm_bindgen_test]
async fn add() {
	let flag = Flag::new();

	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();
	context
		.add_wasm({
			let flag = flag.clone();
			move |_| flag.signal()
		})
		.await
		.unwrap();

	flag.await;
}

/// [`WorkletExt::add_wasm_async()`].
#[wasm_bindgen_test]
async fn add_async() {
	if !worklet::has_async_support() {
		return;
	}

	let flag = Flag::new();

	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();
	context
		.add_wasm_async({
			let flag = flag.clone();
			move |_| async move { flag.signal() }
		})
		.await
		.unwrap();

	flag.await;
}

/// [`WorkletExt::add_wasm()`] returning [`WorkletInitError`].
#[wasm_bindgen_test]
async fn failure() {
	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();

	let flag = Flag::new();
	context
		.add_wasm({
			let flag = flag.clone();
			move |_| flag.signal()
		})
		.await
		.unwrap();

	flag.await;

	let flag = Flag::new();
	let result = WorkletBuilder::new().add(&context, {
		let flag = flag.clone();
		move |_| flag.signal()
	});

	assert_eq!(result.unwrap_err(), WorkletInitError);

	// The worklet will never signal if not re-initialized.
	let result = future::select(flag, util::sleep(SIGNAL_DURATION)).await;
	assert!(matches!(result, Either::Right(((), _))));
}

/// [`WorkletContext::new()`].
#[wasm_bindgen_test]
async fn context() {
	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();

	let flag = Flag::new();
	context
		.add_wasm({
			let flag = flag.clone();
			move |_| {
				WorkletContext::new().unwrap();
				// Flag will never signal if `WorkerContext::new` panics.
				flag.signal();
			}
		})
		.await
		.unwrap();

	flag.await;
}

/// [`WorkletContext::new()`] fails outside worker.
#[wasm_bindgen_test]
fn context_fail() {
	assert!(WorkletContext::new().is_none());
}

/// [`WorkletBuilder::add()`].
#[wasm_bindgen_test]
async fn builder() {
	let flag = Flag::new();

	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();
	WorkletBuilder::new()
		.add(&context, {
			let flag = flag.clone();
			move |_| flag.signal()
		})
		.unwrap()
		.await
		.unwrap();

	flag.await;
}

/// [`WorkletBuilder::add_async()`].
#[wasm_bindgen_test]
async fn builder_async() {
	if !worklet::has_async_support() {
		return;
	}

	let flag = Flag::new();

	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();
	WorkletBuilder::new()
		.add_async(&context, {
			let flag = flag.clone();
			move |_| async move { flag.signal() }
		})
		.unwrap()
		.await
		.unwrap();

	flag.await;
}

/// [`WorkletUrl::new()`], [`WorkletBuilder::new_with_url()`] and
/// [`WorkletBuilder::add()`].
#[wasm_bindgen_test]
async fn builder_url() {
	// We will just use the default `WorkletUrl` but build it ourselves.
	let url = wasm_bindgen::shim_url().unwrap();
	let format = match wasm_bindgen::shim_format().unwrap() {
		wasm_bindgen::ShimFormat::EsModule => ShimFormat::EsModule,
		wasm_bindgen::ShimFormat::NoModules { global_name } => ShimFormat::Classic {
			global: global_name.into(),
		},
		_ => unreachable!("expected shim to be built for browsers"),
	};

	let url = WorkletUrl::new(&url, format).await.unwrap();

	let flag = Flag::new();

	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();
	WorkletBuilder::new_with_url(&url)
		.add(&context, {
			let flag = flag.clone();
			move |_| flag.signal()
		})
		.unwrap()
		.await
		.unwrap();

	flag.await;
}
