#![cfg(test)]
#![cfg(all(target_family = "wasm", feature = "audio-worklet"))]

use std::cell::OnceCell;

use async_channel::Sender;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::{
	AudioContext, AudioWorkletGlobalScope, AudioWorkletNode, AudioWorkletNodeOptions,
	AudioWorkletProcessor, OfflineAudioContext,
};
use web_thread::web::audio_worklet::{
	AudioWorkletGlobalScopeExt, BaseAudioContextExt, ExtendAudioWorkletProcessor,
};
use web_thread::web::{self, YieldTime};

thread_local! {
	static SENDER: OnceCell<Sender<()>> = const { OnceCell::new() };
}

struct TestProcessor;

impl ExtendAudioWorkletProcessor for TestProcessor {
	type Data = Sender<()>;

	fn new(_: AudioWorkletProcessor, data: Option<Self::Data>, _: AudioWorkletNodeOptions) -> Self {
		if let Some(sender) = data {
			sender.try_send(()).unwrap();
		} else {
			SENDER.with(|cell| {
				if let Some(sender) = cell.get() {
					sender.try_send(()).unwrap();
				}
			});
		}

		Self
	}
}

struct ParkProcessor;

impl ExtendAudioWorkletProcessor for ParkProcessor {
	type Data = Sender<()>;

	fn new(_: AudioWorkletProcessor, data: Option<Self::Data>, _: AudioWorkletNodeOptions) -> Self {
		web_thread::park();
		data.unwrap().try_send(()).unwrap();

		Self
	}
}

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
	let (sender, receiver) = async_channel::bounded(1);

	let handle = AudioContext::new()
		.unwrap()
		.register_thread(move || sender.try_send(()).unwrap())
		.await
		.unwrap();

	receiver.recv().await.unwrap();
	// SAFETY: We are sure the thread has spawned by now.
	unsafe { handle.destroy() }
}

#[wasm_bindgen_test]
async fn register_drop() {
	let (sender, receiver) = async_channel::bounded(1);

	AudioContext::new()
		.unwrap()
		.register_thread(move || sender.try_send(()).unwrap());

	receiver.recv().await.unwrap();
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
	let (sender, receiver) = async_channel::bounded(1);

	let handle =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap()
			.register_thread(move || sender.try_send(()).unwrap())
			.await
			.unwrap();

	receiver.recv().await.unwrap();
	// SAFETY: We are sure the thread has spawned by now.
	unsafe { handle.destroy() }
}

#[wasm_bindgen_test]
async fn offline_register_drop() {
	let (sender, receiver) = async_channel::bounded(1);

	OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
		.unwrap()
		.register_thread(move || sender.try_send(()).unwrap());

	receiver.recv().await.unwrap();
}

#[wasm_bindgen_test]
async fn node() {
	let context = AudioContext::new().unwrap();
	let (start_sender, start_receiver) = async_channel::bounded(1);
	let (end_sender, end_receiver) = async_channel::bounded(1);
	context
		.clone()
		.register_thread(move || {
			SENDER.with(|cell| cell.set(end_sender).unwrap());
			let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
			global
				.register_processor_ext::<TestProcessor>("test")
				.unwrap();
			start_sender.try_send(()).unwrap();
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start_receiver.recv().await.unwrap();
	web::yield_now_async(YieldTime::UserBlocking).await;

	AudioWorkletNode::new(&context, "test").unwrap();
	end_receiver.recv().await.unwrap();
}

#[wasm_bindgen_test]
async fn offline_node() {
	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();
	let (start_sender, start_receiver) = async_channel::bounded(1);
	let (end_sender, end_receiver) = async_channel::bounded(1);
	context
		.clone()
		.register_thread(move || {
			SENDER.with(|cell| cell.set(end_sender).unwrap());
			let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
			global
				.register_processor_ext::<TestProcessor>("test")
				.unwrap();
			start_sender.try_send(()).unwrap();
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start_receiver.recv().await.unwrap();
	web::yield_now_async(YieldTime::UserBlocking).await;

	AudioWorkletNode::new(&context, "test").unwrap();
	end_receiver.recv().await.unwrap();
}

#[wasm_bindgen_test]
async fn node_data() {
	let context = AudioContext::new().unwrap();
	let (start_sender, start_receiver) = async_channel::bounded(1);
	let (end_sender, end_receiver) = async_channel::bounded(1);
	context
		.clone()
		.register_thread(move || {
			let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
			global
				.register_processor_ext::<TestProcessor>("test")
				.unwrap();
			start_sender.try_send(()).unwrap();
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start_receiver.recv().await.unwrap();
	web::yield_now_async(YieldTime::UserBlocking).await;

	context
		.audio_worklet_node::<TestProcessor>("test", end_sender, None)
		.unwrap();
	end_receiver.recv().await.unwrap();
}

#[wasm_bindgen_test]
async fn offline_node_data() {
	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();
	let (start_sender, start_receiver) = async_channel::bounded(1);
	let (end_sender, end_receiver) = async_channel::bounded(1);
	context
		.clone()
		.register_thread(move || {
			let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
			global
				.register_processor_ext::<TestProcessor>("test")
				.unwrap();
			start_sender.try_send(()).unwrap();
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start_receiver.recv().await.unwrap();
	web::yield_now_async(YieldTime::UserBlocking).await;

	context
		.audio_worklet_node::<TestProcessor>("test", end_sender, None)
		.unwrap();
	end_receiver.recv().await.unwrap();
}

#[wasm_bindgen_test]
async fn unpark() {
	let context = AudioContext::new().unwrap();
	let (start_sender, start_receiver) = async_channel::bounded(1);
	let (end_sender, end_receiver) = async_channel::bounded(1);
	let handle = context
		.clone()
		.register_thread(move || {
			let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
			global
				.register_processor_ext::<ParkProcessor>("test")
				.unwrap();
			start_sender.try_send(()).unwrap();
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start_receiver.recv().await.unwrap();
	web::yield_now_async(YieldTime::UserBlocking).await;

	handle.thread().unpark();
	context
		.audio_worklet_node::<ParkProcessor>("test", end_sender, None)
		.unwrap();
	end_receiver.recv().await.unwrap();
}

#[wasm_bindgen_test]
async fn offline_unpark() {
	let context =
		OfflineAudioContext::new_with_number_of_channels_and_length_and_sample_rate(1, 1, 8000.)
			.unwrap();
	let (start_sender, start_receiver) = async_channel::bounded(1);
	let (end_sender, end_receiver) = async_channel::bounded(1);
	let handle = context
		.clone()
		.register_thread(move || {
			let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
			global
				.register_processor_ext::<ParkProcessor>("test")
				.unwrap();
			start_sender.try_send(()).unwrap();
		})
		.await
		.unwrap();

	// Wait until processor is registered.
	start_receiver.recv().await.unwrap();
	web::yield_now_async(YieldTime::UserBlocking).await;

	handle.thread().unpark();
	context
		.audio_worklet_node::<ParkProcessor>("test", end_sender, None)
		.unwrap();
	end_receiver.recv().await.unwrap();
}
