#![cfg(test)]
#![cfg(all(target_family = "wasm", feature = "audio-worklet"))]

use std::cell::RefCell;

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::{AudioContext, AudioWorkletGlobalScope, AudioWorkletNode, OfflineAudioContext};
use web_thread::web::audio_worklet::{AudioWorkletGlobalScopeExt, BaseAudioContextExt};
use web_thread::web::{self, YieldTime};

use super::test_processor::{TestProcessor, GLOBAL_DATA};
use super::util::Flag;

#[wasm_bindgen_test]
async fn register() {
	AudioContext::new()
		.unwrap()
		.register_thread(|| ())
		.await
		.unwrap();
}

#[wasm_bindgen_test]
async fn register_destroy() {
	let flag = Flag::new();

	let handle = AudioContext::new()
		.unwrap()
		.register_thread({
			let flag = flag.clone();
			move || flag.signal()
		})
		.await
		.unwrap();

	flag.await;
	// SAFETY: We are sure the thread has spawned by now.
	unsafe { handle.destroy() }
}

#[wasm_bindgen_test]
async fn register_drop() {
	let flag = Flag::new();

	AudioContext::new().unwrap().register_thread({
		let flag = flag.clone();
		move || flag.signal()
	});

	flag.await;
}

#[wasm_bindgen_test]
async fn offline_register() {
	OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
		.unwrap()
		.register_thread(|| ())
		.await
		.unwrap();
}

#[wasm_bindgen_test]
async fn offline_register_destroy() {
	let flag = Flag::new();

	let handle =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap()
			.register_thread({
				let flag = flag.clone();
				move || flag.signal()
			})
			.await
			.unwrap();

	flag.await;
	// SAFETY: We are sure the thread has spawned by now.
	unsafe { handle.destroy() }
}

#[wasm_bindgen_test]
async fn offline_register_drop() {
	let flag = Flag::new();

	OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
		.unwrap()
		.register_thread({
			let flag = flag.clone();
			move || flag.signal()
		});

	flag.await;
}

#[wasm_bindgen_test]
async fn node() {
	let context = AudioContext::new().unwrap();

	let start = Flag::new();
	let end = Flag::new();
	context
		.clone()
		.register_thread({
			let start = start.clone();
			let end = end.clone();
			move || {
				GLOBAL_DATA.with(move |data| {
					#[allow(clippy::blocks_in_conditions)]
					if data
						.set(RefCell::new(Some(Box::new(move || {
							end.signal();
							None
						}))))
						.is_err()
					{
						panic!()
					}
				});
				let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
				global
					.register_processor_ext::<TestProcessor>("test")
					.unwrap();
				start.signal();
			}
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start.await;
	web::yield_now_async(YieldTime::UserBlocking).await;

	AudioWorkletNode::new(&context, "test").unwrap();
	end.await;
}

#[wasm_bindgen_test]
async fn offline_node() {
	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();

	let start = Flag::new();
	let end = Flag::new();
	context
		.clone()
		.register_thread({
			let start = start.clone();
			let end = end.clone();
			move || {
				GLOBAL_DATA.with(move |data| {
					#[allow(clippy::blocks_in_conditions)]
					if data
						.set(RefCell::new(Some(Box::new(move || {
							end.signal();
							None
						}))))
						.is_err()
					{
						panic!()
					}
				});
				let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
				global
					.register_processor_ext::<TestProcessor>("test")
					.unwrap();
				start.signal();
			}
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start.await;
	web::yield_now_async(YieldTime::UserBlocking).await;

	AudioWorkletNode::new(&context, "test").unwrap();
	end.await;
}

#[wasm_bindgen_test]
async fn node_data() {
	let context = AudioContext::new().unwrap();

	let start = Flag::new();
	context
		.clone()
		.register_thread({
			let start = start.clone();
			move || {
				let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
				global
					.register_processor_ext::<TestProcessor>("test")
					.unwrap();
				start.signal();
			}
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start.await;
	web::yield_now_async(YieldTime::UserBlocking).await;

	let end = Flag::new();
	context
		.audio_worklet_node::<TestProcessor>(
			"test",
			Box::new({
				let end = end.clone();
				move || {
					end.signal();
					None
				}
			}),
			None,
		)
		.unwrap();
	end.await;
}

#[wasm_bindgen_test]
async fn offline_node_data() {
	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();

	let start = Flag::new();
	context
		.clone()
		.register_thread({
			let start = start.clone();
			move || {
				let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
				global
					.register_processor_ext::<TestProcessor>("test")
					.unwrap();
				start.signal();
			}
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start.await;
	web::yield_now_async(YieldTime::UserBlocking).await;

	let end = Flag::new();
	context
		.audio_worklet_node::<TestProcessor>(
			"test",
			Box::new({
				let end = end.clone();
				move || {
					end.signal();
					None
				}
			}),
			None,
		)
		.unwrap();
	end.await;
}

#[wasm_bindgen_test]
async fn unpark() {
	let context = AudioContext::new().unwrap();

	let start = Flag::new();
	let handle = context
		.clone()
		.register_thread({
			let start = start.clone();
			move || {
				let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
				global
					.register_processor_ext::<TestProcessor>("test")
					.unwrap();
				start.signal();
			}
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start.await;
	web::yield_now_async(YieldTime::UserBlocking).await;

	handle.thread().unpark();
	let end = Flag::new();
	context
		.audio_worklet_node::<TestProcessor>(
			"test",
			Box::new({
				let end = end.clone();
				move || {
					web_thread::park();
					end.signal();
					None
				}
			}),
			None,
		)
		.unwrap();
	end.await;
}

#[wasm_bindgen_test]
async fn offline_unpark() {
	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();

	let start = Flag::new();
	let handle = context
		.clone()
		.register_thread({
			let start = start.clone();
			move || {
				let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
				global
					.register_processor_ext::<TestProcessor>("test")
					.unwrap();
				start.signal();
			}
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start.await;
	web::yield_now_async(YieldTime::UserBlocking).await;

	handle.thread().unpark();
	let end = Flag::new();
	context
		.audio_worklet_node::<TestProcessor>(
			"test",
			Box::new({
				let end = end.clone();
				move || {
					web_thread::park();
					end.signal();
					None
				}
			}),
			None,
		)
		.unwrap();
	end.await;
}

#[wasm_bindgen_test]
#[cfg(not(unsupported_headless_audiocontext))]
async fn process() {
	let context = AudioContext::new().unwrap();

	let start = Flag::new();
	let end = Flag::new();
	context
		.clone()
		.register_thread({
			let start = start.clone();
			let end = end.clone();
			move || {
				GLOBAL_DATA.with(move |data| {
					if data
						.set(RefCell::new(Some(Box::new(move || {
							Some(Box::new(move || end.signal()))
						}))))
						.is_err()
					{
						panic!()
					}
				});
				let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
				global
					.register_processor_ext::<TestProcessor>("test")
					.unwrap();
				start.signal();
			}
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start.await;
	web::yield_now_async(YieldTime::UserBlocking).await;

	AudioWorkletNode::new(&context, "test").unwrap();
	end.await;
}

#[wasm_bindgen_test]
async fn offline_process() {
	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();

	let start = Flag::new();
	let end = Flag::new();
	context
		.clone()
		.register_thread({
			let start = start.clone();
			let end = end.clone();
			move || {
				GLOBAL_DATA.with(move |data| {
					if data
						.set(RefCell::new(Some(Box::new(move || {
							Some(Box::new(move || end.signal()))
						}))))
						.is_err()
					{
						panic!()
					}
				});
				let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
				global
					.register_processor_ext::<TestProcessor>("test")
					.unwrap();
				start.signal();
			}
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start.await;
	web::yield_now_async(YieldTime::UserBlocking).await;

	AudioWorkletNode::new(&context, "test").unwrap();
	JsFuture::from(context.start_rendering().unwrap())
		.await
		.unwrap();
	end.await;
}
