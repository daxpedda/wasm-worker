//! Tests audio worklet functionality.

mod util;

use futures_util::future::{self, Either};
use util::Flag;
use wasm_bindgen_test::wasm_bindgen_test;
use wasm_worker::common::ShimFormat;
use wasm_worker::worklet::audio::AudioWorkletUrl;
use wasm_worker::worklet::{WorkletInitError, WorkletModule};
use wasm_worker::AudioWorkletExt;
use web_sys::OfflineAudioContext;

use crate::util::SIGNAL_DURATION;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// [`AudioWorkletExt::init_wasm`].
#[wasm_bindgen_test]
async fn basic() {
	let flag = Flag::new();

	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();
	context
		.init_wasm({
			let flag = flag.clone();
			move |_| flag.signal()
		})
		.unwrap()
		.await
		.unwrap();

	flag.await;
}

/// [`AudioWorkletExt::init_wasm`] returning [`WorkletInitError`].
#[wasm_bindgen_test]
async fn failure() {
	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();

	let flag = Flag::new();
	context
		.init_wasm({
			let flag = flag.clone();
			move |_| flag.signal()
		})
		.unwrap()
		.await
		.unwrap();

	flag.await;

	let flag = Flag::new();
	let result = context.init_wasm({
		let flag = flag.clone();
		move |_| flag.signal()
	});

	assert_eq!(result.unwrap_err(), WorkletInitError);

	// The worklet will never signal if not re-initialized.
	let result = future::select(flag, util::sleep(SIGNAL_DURATION)).await;
	assert!(matches!(result, Either::Right(((), _))));
}

/// [`WorkletModule::new()`], [`AudioWorkletUrl::new()`] and
/// [`AudioWorkletExt::init_wasm_with_url()`].
#[wasm_bindgen_test]
async fn url() {
	// We will just use the default `WorkletModule` but build it ourselves.
	let url = wasm_bindgen::shim_url().unwrap();
	let format = match wasm_bindgen::shim_format().unwrap() {
		wasm_bindgen::ShimFormat::EsModule => ShimFormat::EsModule,
		wasm_bindgen::ShimFormat::NoModules { global_name } => ShimFormat::Classic {
			global: global_name.into(),
		},
		_ => unreachable!("expected shim to be built for browsers"),
	};

	let module = WorkletModule::new(&url, format).await.unwrap();

	let url = AudioWorkletUrl::new(&module);

	let flag = Flag::new();

	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();
	context
		.init_wasm_with_url(&url, {
			let flag = flag.clone();
			move |_| flag.signal()
		})
		.unwrap()
		.await
		.unwrap();

	flag.await;
}
