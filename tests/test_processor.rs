#![cfg(all(target_family = "wasm", feature = "audio-worklet"))]
#![allow(
	missing_copy_implementations,
	missing_debug_implementations,
	unreachable_pub
)]

use std::cell::{OnceCell, RefCell};

use js_sys::{Array, Object};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsCast;
use web_sys::{AudioWorkletNodeOptions, AudioWorkletProcessor};
use web_thread::web::audio_worklet::ExtendAudioWorkletProcessor;

thread_local! {
	#[allow(clippy::type_complexity)]
	pub static GLOBAL_DATA: OnceCell<RefCell<Option<Box<dyn FnOnce(AudioWorkletNodeOptionsExt2) -> Option<Box<dyn FnOnce()>>>>>> = OnceCell::new();
}

pub struct TestProcessor(Option<Box<dyn FnOnce()>>);

impl ExtendAudioWorkletProcessor for TestProcessor {
	type Data = Box<dyn FnOnce(AudioWorkletNodeOptionsExt2) -> Option<Box<dyn FnOnce()>> + Send>;

	fn new(
		_: AudioWorkletProcessor,
		data: Option<Self::Data>,
		options: AudioWorkletNodeOptions,
	) -> Self {
		if let Some(data) =
			GLOBAL_DATA.with(|data| data.get().and_then(|data| data.borrow_mut().take()))
		{
			Self(data(options.unchecked_into()))
		} else if let Some(data) = data {
			Self(data(options.unchecked_into()))
		} else {
			Self(None)
		}
	}

	fn process(&mut self, _: Array, _: Array, _: Object) -> bool {
		if let Some(fun) = self.0.take() {
			fun();
		}

		false
	}
}

#[wasm_bindgen]
extern "C" {
	#[wasm_bindgen(extends = AudioWorkletNodeOptions)]
	#[derive(Default)]
	pub type AudioWorkletNodeOptionsExt2;

	#[wasm_bindgen(getter, method, js_name = processorOptions)]
	pub fn get_processor_options(this: &AudioWorkletNodeOptionsExt2) -> Option<Object>;

	#[wasm_bindgen(setter, method, js_name = processorOptions)]
	pub fn set_processor_options(this: &AudioWorkletNodeOptionsExt2, value: Option<&Object>);
}

impl AudioWorkletNodeOptionsExt2 {
	#[must_use]
	pub fn new() -> Self {
		Object::new().unchecked_into()
	}
}
