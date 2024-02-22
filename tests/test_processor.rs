#![cfg(all(target_family = "wasm", feature = "audio-worklet"))]
#![allow(
	missing_copy_implementations,
	missing_debug_implementations,
	unreachable_pub
)]

use std::cell::{OnceCell, RefCell};

use web_sys::{AudioWorkletNodeOptions, AudioWorkletProcessor};
use web_thread::web::audio_worklet::ExtendAudioWorkletProcessor;

thread_local! {
	#[allow(clippy::type_complexity)]
	pub static GLOBAL_DATA: OnceCell<RefCell<Option<Box<dyn FnOnce()>>>> = OnceCell::new();
}

pub struct TestProcessor;

impl ExtendAudioWorkletProcessor for TestProcessor {
	type Data = Box<dyn FnOnce() + Send>;

	fn new(_: AudioWorkletProcessor, data: Option<Self::Data>, _: AudioWorkletNodeOptions) -> Self {
		if let Some(data) =
			GLOBAL_DATA.with(|data| data.get().and_then(|data| data.borrow_mut().take()))
		{
			data();
		} else if let Some(data) = data {
			data();
		}

		Self
	}
}
