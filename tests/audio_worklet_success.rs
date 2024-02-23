#![cfg(test)]
#![cfg(all(target_family = "wasm", feature = "audio-worklet"))]

use std::cell::RefCell;

use js_sys::{Array, Iterator, Object, Reflect};
use utf32_lit::utf32;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::{
	AudioContext, AudioWorkletGlobalScope, AudioWorkletNode, AudioWorkletNodeOptions,
	OfflineAudioContext,
};
use web_thread::web::audio_worklet::{AudioWorkletGlobalScopeExt, BaseAudioContextExt};
use web_thread::web::{self, YieldTime};

use super::test_processor::{
	AudioParameter, AudioWorkletNodeOptionsExt2, TestProcessor, GLOBAL_DATA,
};
use super::util::Flag;
use crate::js_string;

#[wasm_bindgen_test]
async fn register() {
	AudioContext::new()
		.unwrap()
		.register_thread(|| ())
		.await
		.unwrap();
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
async fn register_drop() {
	let flag = Flag::new();

	AudioContext::new().unwrap().register_thread({
		let flag = flag.clone();
		move || flag.signal()
	});

	flag.await;
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
						.set(RefCell::new(Some(Box::new(move |_| {
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
						.set(RefCell::new(Some(Box::new(move |_| {
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
				move |_| {
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
				move |_| {
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
				move |_| {
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
				move |_| {
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
						.set(RefCell::new(Some(Box::new(move |_| {
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
						.set(RefCell::new(Some(Box::new(move |_| {
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

#[wasm_bindgen_test]
async fn no_options() {
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
						.set(RefCell::new(Some(Box::new(move |options| {
							assert!(options.get_processor_options().is_none());
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
async fn offline_no_options() {
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
						.set(RefCell::new(Some(Box::new(move |options| {
							assert!(options.get_processor_options().is_none());
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
async fn zero_options() {
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
						.set(RefCell::new(Some(Box::new(move |options| {
							assert_eq!(
								Object::keys(&options.get_processor_options().unwrap()).length(),
								0
							);
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

	let options = AudioWorkletNodeOptionsExt2::new();
	options.set_processor_options(Some(&Object::new()));
	AudioWorkletNode::new_with_options(&context, "test", &options).unwrap();
	end.await;
}

#[wasm_bindgen_test]
async fn offline_zero_options() {
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
						.set(RefCell::new(Some(Box::new(move |options| {
							assert_eq!(
								Object::keys(&options.get_processor_options().unwrap()).length(),
								0
							);
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

	let options = AudioWorkletNodeOptionsExt2::new();
	options.set_processor_options(Some(&Object::new()));
	AudioWorkletNode::new_with_options(&context, "test", &options).unwrap();
	end.await;
}

#[wasm_bindgen_test]
async fn data_no_options() {
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
				move |options| {
					assert!(options.get_processor_options().is_none());
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
async fn offline_data_no_options() {
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
				move |options| {
					assert!(options.get_processor_options().is_none());
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
async fn data_empty_options() {
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
	let options = AudioWorkletNodeOptionsExt2::new();
	context
		.audio_worklet_node::<TestProcessor>(
			"test",
			Box::new({
				let end = end.clone();
				move |options| {
					assert!(options.get_processor_options().is_none());
					end.signal();
					None
				}
			}),
			Some(&options),
		)
		.unwrap();
	assert!(options.get_processor_options().is_none());
	end.await;
}

#[wasm_bindgen_test]
async fn offline_data_empty_options() {
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
	let options = AudioWorkletNodeOptionsExt2::new();
	context
		.audio_worklet_node::<TestProcessor>(
			"test",
			Box::new({
				let end = end.clone();
				move |options| {
					assert!(options.get_processor_options().is_none());
					end.signal();
					None
				}
			}),
			Some(&options),
		)
		.unwrap();
	assert!(options.get_processor_options().is_none());
	end.await;
}

#[wasm_bindgen_test]
async fn data_zero_options() {
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
	let options = AudioWorkletNodeOptionsExt2::new();
	options.set_processor_options(Some(&Object::new()));
	context
		.audio_worklet_node::<TestProcessor>(
			"test",
			Box::new({
				let end = end.clone();
				move |options| {
					assert_eq!(
						Object::keys(&options.get_processor_options().unwrap()).length(),
						0
					);
					end.signal();
					None
				}
			}),
			Some(&options),
		)
		.unwrap();
	assert_eq!(
		Object::keys(&options.get_processor_options().unwrap()).length(),
		0
	);
	end.await;
}

#[wasm_bindgen_test]
async fn offline_data_zero_options() {
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
	let options = AudioWorkletNodeOptionsExt2::new();
	options.set_processor_options(Some(&Object::new()));
	context
		.audio_worklet_node::<TestProcessor>(
			"test",
			Box::new({
				let end = end.clone();
				move |options| {
					assert_eq!(
						Object::keys(&options.get_processor_options().unwrap()).length(),
						0
					);
					end.signal();
					None
				}
			}),
			Some(&options),
		)
		.unwrap();
	assert_eq!(
		Object::keys(&options.get_processor_options().unwrap()).length(),
		0
	);
	end.await;
}

#[wasm_bindgen_test]
async fn options() {
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
					#[allow(clippy::blocks_in_conditions, clippy::float_cmp)]
					if data
						.set(RefCell::new(Some(Box::new(move |options| {
							let options: Object = options.get_processor_options().unwrap();
							let var = Reflect::get_u32(&options, 0).unwrap();
							assert_eq!(var, 42.);
							assert_eq!(Object::keys(&options).length(), 1);
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

	let options = Object::new();
	Reflect::set_u32(&options, 0, &42.into()).unwrap();
	AudioWorkletNode::new_with_options(
		&context,
		"test",
		AudioWorkletNodeOptions::new().processor_options(Some(&options)),
	)
	.unwrap();
	end.await;
}

#[wasm_bindgen_test]
async fn offline_options() {
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
					#[allow(clippy::blocks_in_conditions, clippy::float_cmp)]
					if data
						.set(RefCell::new(Some(Box::new(move |options| {
							let options: Object = options.get_processor_options().unwrap();
							let var = Reflect::get_u32(&options, 0).unwrap();
							assert_eq!(var, 42.);
							assert_eq!(Object::keys(&options).length(), 1);
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

	let options = Object::new();
	Reflect::set_u32(&options, 0, &42.into()).unwrap();
	AudioWorkletNode::new_with_options(
		&context,
		"test",
		AudioWorkletNodeOptions::new().processor_options(Some(&options)),
	)
	.unwrap();
	end.await;
}

#[wasm_bindgen_test]
async fn options_data() {
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
	let inner_options = Object::new();
	Reflect::set_u32(&inner_options, 0, &42.into()).unwrap();
	let mut options = AudioWorkletNodeOptions::new();
	options.processor_options(Some(&inner_options));
	context
		.audio_worklet_node::<TestProcessor>(
			"test",
			Box::new({
				let end = end.clone();
				#[allow(clippy::float_cmp)]
				move |options| {
					let options: Object = options.get_processor_options().unwrap();
					let var = Reflect::get_u32(&options, 0).unwrap();
					assert_eq!(var, 42.);
					assert_eq!(Object::keys(&options).length(), 1);
					end.signal();
					None
				}
			}),
			Some(&options),
		)
		.unwrap();
	assert_eq!(Object::keys(&inner_options).length(), 1);
	end.await;
}

#[wasm_bindgen_test]
async fn offline_options_data() {
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
	let inner_options = Object::new();
	Reflect::set_u32(&inner_options, 0, &42.into()).unwrap();
	let mut options = AudioWorkletNodeOptions::new();
	options.processor_options(Some(&inner_options));
	context
		.audio_worklet_node::<TestProcessor>(
			"test",
			Box::new({
				let end = end.clone();
				#[allow(clippy::float_cmp)]
				move |options| {
					let options: Object = options.get_processor_options().unwrap();
					let var = Reflect::get_u32(&options, 0).unwrap();
					assert_eq!(var, 42.);
					assert_eq!(Object::keys(&options).length(), 1);
					end.signal();
					None
				}
			}),
			Some(&options),
		)
		.unwrap();
	assert_eq!(Object::keys(&inner_options).length(), 1);
	end.await;
}

struct TestParameters;

impl AudioParameter for TestParameters {
	fn parameter_descriptors() -> Iterator {
		let parameters = Array::new();

		let parameter = Object::new();
		Reflect::set(&parameter, &js_string!("name"), &js_string!("test")).unwrap();

		parameters.push(&parameter);
		parameters.values()
	}
}

#[wasm_bindgen_test]
async fn parameters() {
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
						.set(RefCell::new(Some(Box::new(move |options| {
							let parameters = options.get_parameter_data().unwrap();
							let value = Reflect::get(&parameters, &js_string!("test")).unwrap();
							assert_eq!(value, 42.);
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
					.register_processor_ext::<TestProcessor<TestParameters>>("test")
					.unwrap();

				start.signal();
			}
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start.await;
	web::yield_now_async(YieldTime::UserBlocking).await;

	let options = AudioWorkletNodeOptionsExt2::new();
	let parameters = Array::new();
	Reflect::set(&parameters, &js_string!("test"), &42.0.into()).unwrap();
	options.set_parameter_data(Some(&parameters));
	AudioWorkletNode::new_with_options(&context, "test", &options).unwrap();
	end.await;
}

#[wasm_bindgen_test]
async fn offline_parameters() {
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
						.set(RefCell::new(Some(Box::new(move |options| {
							let parameters = options.get_parameter_data().unwrap();
							let value = Reflect::get(&parameters, &js_string!("test")).unwrap();
							assert_eq!(value, 42.);
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
					.register_processor_ext::<TestProcessor<TestParameters>>("test")
					.unwrap();

				start.signal();
			}
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start.await;
	web::yield_now_async(YieldTime::UserBlocking).await;

	let options = AudioWorkletNodeOptionsExt2::new();
	let parameters = Array::new();
	Reflect::set(&parameters, &js_string!("test"), &42.0.into()).unwrap();
	options.set_parameter_data(Some(&parameters));
	AudioWorkletNode::new_with_options(&context, "test", &options).unwrap();
	end.await;
}
