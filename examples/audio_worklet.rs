//! Audio worklet example.

#![allow(clippy::unwrap_used)]

#[cfg(not(target_family = "wasm"))]
fn main() {
	panic!("This example is supposed to only be run with the `wasm32-unknown-unknown` target.")
}

#[cfg(target_family = "wasm")]
fn main() {
	self::web::main();
}

/// Implementation for the Web platform.
#[cfg(target_family = "wasm")]
mod web {
	use js_sys::{Array, Object};
	use wasm_bindgen::closure::Closure;
	use wasm_bindgen::{JsCast, JsValue};
	use wasm_bindgen_futures::JsFuture;
	use web_sys::{
		console, AudioContext, AudioWorkletGlobalScope, AudioWorkletNode, AudioWorkletNodeOptions,
		AudioWorkletProcessor, Blob, BlobPropertyBag, HtmlButtonElement, HtmlHtmlElement, Url,
	};
	use web_thread::web::audio_worklet::{
		AudioWorkletGlobalScopeExt, BaseAudioContextExt, ExtendAudioWorkletProcessor,
	};
	use web_thread::web::{self, YieldTime};

	/// `fn main` implementation.
	pub(crate) fn main() {
		console_error_panic_hook::set_once();

		// Make it possible center elements on the screen.
		let window = web_sys::window().unwrap();
		let document = window.document().unwrap();
		let body = document.body().unwrap();
		document
			.document_element()
			.unwrap()
			.unchecked_into::<HtmlHtmlElement>()
			.style()
			.set_property("height", "100%")
			.unwrap();
		let style = body.style();
		style.set_property("height", "100%").unwrap();
		style.set_property("display", "grid").unwrap();

		// Create centered button.
		let button: HtmlButtonElement = document.create_element("button").unwrap().unchecked_into();
		button.style().set_property("margin", "auto").unwrap();
		button.set_inner_text("Start");
		body.append_child(&button).unwrap();

		// Let button start the audio worklet.
		button.clone().set_onclick(Some(
			Closure::once_into_js(|| {
				// Remove button after starting.
				button.set_disabled(true);
				button.set_onclick(None);

				wasm_bindgen_futures::future_to_promise(async {
					start(button).await;
					Ok(JsValue::UNDEFINED)
				})
			})
			.as_ref()
			.unchecked_ref(),
		));
	}

	/// We can only start an [`AudioContext`] after a user-interaction.
	async fn start(button: HtmlButtonElement) {
		let context = AudioContext::new().unwrap();

		// Firefox requires a polyfill for `TextDecoder`/`TextEncoder`: <https://bugzilla.mozilla.org/show_bug.cgi?id=1826432>
		JsFuture::from(
			context
				.audio_worklet()
				.unwrap()
				.add_module(&url(include_str!("polyfill.js")))
				.unwrap(),
		)
		.await
		.unwrap();

		// Register thread.
		let (sender, receiver) = async_channel::bounded(1);
		context
			.clone()
			.register_thread(move || {
				console::log_1(&"Hello from audio worklet!".into());

				let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();
				// Register `TestProcessor`.
				global
					.register_processor_ext::<TestProcessor>("test")
					.unwrap();
				sender.try_send(()).unwrap();
			})
			.await
			.unwrap();

		// Wait until processor is registered.
		receiver.recv().await.unwrap();
		web::yield_now_async(YieldTime::UserBlocking).await;

		// Initialize `TestProcessor`.
		AudioWorkletNode::new(&context, "test").unwrap();

		button.set_onclick(Some(
			Closure::once_into_js({
				let button = button.clone();
				move || {
					button.remove();
					context.close().unwrap()
				}
			})
			.as_ref()
			.unchecked_ref(),
		));
		button.set_inner_text("Stop");
		button.set_disabled(false);
	}

	/// Create an object URL from a JS script.
	fn url(script: &str) -> String {
		let sequence = Array::of1(&script.into());
		let mut property = BlobPropertyBag::new();
		property.type_("text/javascript");
		let blob = Blob::new_with_str_sequence_and_options(&sequence, &property)
			.expect("`new Blob()` should never throw");

		Url::create_object_url_with_blob(&blob).expect("`URL.createObjectURL()` should never throw")
	}

	/// Example [`AudioWorkletProcessor`].
	struct TestProcessor;

	impl ExtendAudioWorkletProcessor for TestProcessor {
		type Data = ();

		fn new(
			_: AudioWorkletProcessor,
			_: Option<Self::Data>,
			_: AudioWorkletNodeOptions,
		) -> Self {
			console::log_1(&"`TestProcessor` initialized!".into());
			Self
		}

		fn process(&mut self, _: Array, _: Array, _: Object) -> bool {
			console::log_1(&"Hello from `TestProcessor`!".into());
			false
		}
	}
}
