#![cfg(all(target_family = "wasm", feature = "audio-worklet"))]
#![allow(
	missing_copy_implementations,
	missing_debug_implementations,
	unreachable_pub
)]

use std::cell::{OnceCell, RefCell};

use js_sys::{Array, Object};
use web_sys::{AudioWorkletNodeOptions, AudioWorkletProcessor};
use web_thread::web::audio_worklet::ExtendAudioWorkletProcessor;

thread_local! {
	#[allow(clippy::type_complexity)]
	pub static GLOBAL_DATA: OnceCell<RefCell<Option<Box<dyn FnOnce() -> Option<Box<dyn FnOnce()>>>>>> = OnceCell::new();
}

pub struct TestProcessor(Option<Box<dyn FnOnce()>>);

impl ExtendAudioWorkletProcessor for TestProcessor {
	type Data = Box<dyn FnOnce() -> Option<Box<dyn FnOnce()>> + Send>;

	fn new(_: AudioWorkletProcessor, data: Option<Self::Data>, _: AudioWorkletNodeOptions) -> Self {
		if let Some(data) =
			GLOBAL_DATA.with(|data| data.get().and_then(|data| data.borrow_mut().take()))
		{
			Self(data())
		} else if let Some(data) = data {
			Self(data())
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
