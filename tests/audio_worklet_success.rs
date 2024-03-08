#![cfg(test)]
#![cfg(all(target_family = "wasm", feature = "audio-worklet"))]

use std::cell::RefCell;
use std::future::Future;

use js_sys::{Array, Iterator, Object, Reflect};
use paste::paste;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::{
	AudioContext, AudioWorkletGlobalScope, AudioWorkletNode, AudioWorkletNodeOptions,
	BaseAudioContext, OfflineAudioContext,
};
use web_thread::web::audio_worklet::{AudioWorkletGlobalScopeExt, BaseAudioContextExt};
use web_thread::web::{self, YieldTime};

use super::test_processor::{
	AudioParameter, AudioWorkletNodeOptionsExt2, TestProcessor, GLOBAL_DATA,
};
use super::util::Flag;
use crate::js_string;

macro_rules! test {
	($name:ident) => {
		paste! {
			#[wasm_bindgen_test]
			async fn $name() {
				[<test_ $name>](AudioContext::new().unwrap().into()).await;
			}

			#[wasm_bindgen_test]
			async fn [<offline_ $name>]() {
				[<test_ $name>](
					OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(
						1, 1, 8000.,
					)
					.unwrap()
					.into(),
				)
				.await;
			}
		}
	};
}

async fn test_nested<C, F>(context: C, post: impl FnOnce(C) -> F)
where
	C: Clone + BaseAudioContextExt + AsRef<BaseAudioContext>,
	F: Future<Output = ()>,
{
	let start = Flag::new();
	let end = Flag::new();
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

	context
		.audio_worklet_node::<TestProcessor>(
			"test",
			Box::new({
				let end = end.clone();
				move |_| {
					let handle = web_thread::spawn(|| ());
					Some(Box::new(move || {
						if handle.is_finished() {
							end.signal();
							false
						} else {
							true
						}
					}))
				}
			}),
			None,
		)
		.unwrap();
	post(context).await;
	end.await;
}

#[wasm_bindgen_test]
// Firefox doesn't support running `AudioContext` without an actual audio device.
// See <https://bugzilla.mozilla.org/show_bug.cgi?id=1881904>.
#[cfg(not(unsupported_headless_audiocontext))]
async fn nested() {
	test_nested(AudioContext::new().unwrap(), |_| async {}).await;
}

#[wasm_bindgen_test]
async fn offline_nested() {
	test_nested(
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(
			1,
			100_000_000,
			8000.,
		)
		.unwrap(),
		|context| async move {
			JsFuture::from(context.start_rendering().unwrap())
				.await
				.unwrap();
		},
	)
	.await;
}

async fn test_register(context: BaseAudioContext) {
	context.register_thread(|| ()).await.unwrap();
}

test!(register);

async fn test_register_release(context: BaseAudioContext) {
	let flag = Flag::new();

	let handle = context
		.register_thread({
			let flag = flag.clone();
			move || flag.signal()
		})
		.await
		.unwrap();

	flag.await;
	// SAFETY: We are sure the thread has spawned by now and we also didn't register
	// any events or promises that could call into the Wasm module later.
	unsafe { handle.release() }.unwrap();
}

test!(register_release);

async fn test_register_drop(context: BaseAudioContext) {
	let flag = Flag::new();

	context.register_thread({
		let flag = flag.clone();
		move || flag.signal()
	});

	flag.await;
}

test!(register_drop);

async fn test_node(context: BaseAudioContext) {
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

test!(node);

async fn test_node_data(context: BaseAudioContext) {
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

test!(node_data);

async fn test_unpark(context: BaseAudioContext) {
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

test!(unpark);

async fn test_process<C, F>(context: C, post: impl FnOnce(C) -> F)
where
	C: Clone + BaseAudioContextExt + AsRef<BaseAudioContext>,
	F: Future<Output = ()>,
{
	let start = Flag::new();
	let end = Flag::new();
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

	context
		.audio_worklet_node::<TestProcessor>(
			"test",
			Box::new({
				let end = end.clone();
				move |_| {
					Some(Box::new(move || {
						end.signal();
						false
					}))
				}
			}),
			None,
		)
		.unwrap();
	post(context).await;
	end.await;
}

#[wasm_bindgen_test]
// Firefox doesn't support running `AudioContext` without an actual audio device.
// See <https://bugzilla.mozilla.org/show_bug.cgi?id=1881904>.
#[cfg(not(unsupported_headless_audiocontext))]
async fn process() {
	test_process(AudioContext::new().unwrap(), |_| async {}).await;
}

#[wasm_bindgen_test]
async fn offline_process() {
	test_process(
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap(),
		|context| async move {
			JsFuture::from(context.start_rendering().unwrap())
				.await
				.unwrap();
		},
	)
	.await;
}

async fn test_no_options(context: BaseAudioContext) {
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

test!(no_options);

async fn test_zero_options(context: BaseAudioContext) {
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

test!(zero_options);

async fn test_data_no_options(context: BaseAudioContext) {
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

test!(data_no_options);

async fn test_data_empty_options(context: BaseAudioContext) {
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

test!(data_empty_options);

async fn test_data_zero_options(context: BaseAudioContext) {
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

test!(data_zero_options);

async fn test_options(context: BaseAudioContext) {
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

test!(options);

async fn test_options_data(context: BaseAudioContext) {
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

test!(options_data);

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

async fn test_parameters(context: BaseAudioContext) {
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

test!(parameters);
