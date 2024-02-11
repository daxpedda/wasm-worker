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
	use std::cell::Cell;
	use std::rc::Rc;

	use itertools::Itertools;
	use js_sys::{Array, Float32Array, Object};
	use wasm_bindgen::closure::Closure;
	use wasm_bindgen::{JsCast, JsValue};
	use wasm_bindgen_futures::JsFuture;
	use web_sys::{
		console, AudioContext, AudioWorkletGlobalScope, AudioWorkletNode, AudioWorkletNodeOptions,
		AudioWorkletProcessor, Blob, BlobPropertyBag, ChannelMergerNode, ChannelMergerOptions,
		ChannelSplitterNode, ChannelSplitterOptions, Document, Event, GainNode, GainOptions,
		HtmlButtonElement, HtmlElement, HtmlHtmlElement, HtmlInputElement, HtmlLabelElement,
		HtmlTableElement, HtmlTableRowElement, Url,
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

		// Create centered container.
		let container: HtmlElement = document.create_element("div").unwrap().unchecked_into();
		let style = container.style();
		style.set_property("margin", "auto").unwrap();
		style.set_property("text-align", "center").unwrap();
		body.append_child(&container).unwrap();

		// Create start/end button.
		let button: HtmlButtonElement = document.create_element("button").unwrap().unchecked_into();
		button.set_inner_text("Start");
		container.append_child(&button).unwrap();

		// Let button start the audio worklet.
		button.clone().set_onclick(Some(
			Closure::once_into_js(|| {
				// Remove button after starting.
				button.remove();
				button.set_onclick(None);

				wasm_bindgen_futures::future_to_promise(async {
					start(document, container, button).await;
					Ok(JsValue::UNDEFINED)
				})
			})
			.as_ref()
			.unchecked_ref(),
		));
	}

	/// We can only start an [`AudioContext`] after a user-interaction.
	#[allow(clippy::too_many_lines)]
	async fn start(document: Document, container: HtmlElement, button: HtmlButtonElement) {
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
				// Register `ExampleProcessor`.
				global
					.register_processor_ext::<ExampleProcessor>("example")
					.unwrap();
				sender.try_send(()).unwrap();
			})
			.await
			.unwrap();

		// Wait until processor is registered.
		receiver.recv().await.unwrap();
		web::yield_now_async(YieldTime::UserBlocking).await;

		let channel_count = context.destination().channel_count();

		// Initialize `ExampleProcessor`.
		let mut options = AudioWorkletNodeOptions::new();
		options.output_channel_count(&Array::of1(&channel_count.into()));
		let worklet = AudioWorkletNode::new_with_options(&context, "example", &options).unwrap();

		// Create channel splitter node.
		let mut options = ChannelSplitterOptions::new();
		options.number_of_outputs(channel_count);
		let channel_splitter = ChannelSplitterNode::new_with_options(&context, &options).unwrap();
		worklet.connect_with_audio_node(&channel_splitter).unwrap();

		// Create channel merger node.
		let mut options = ChannelMergerOptions::new();
		options.number_of_inputs(channel_count);
		let channel_merger = ChannelMergerNode::new_with_options(&context, &options).unwrap();
		channel_merger
			.connect_with_audio_node(&context.destination())
			.unwrap();

		// Create table to present slider for each channel.
		let table: HtmlTableElement = document.create_element("table").unwrap().unchecked_into();
		container.append_child(&table).unwrap();
		let style = table.style();
		style.set_property("border", "1px solid").unwrap();
		style.set_property("border-collapse", "collapse").unwrap();
		let name_row: HtmlTableRowElement = table.insert_row().unwrap().unchecked_into();
		let input_row: HtmlTableRowElement = table.insert_row().unwrap().unchecked_into();
		let label_row: HtmlTableRowElement = table.insert_row().unwrap().unchecked_into();

		// Create master volume control.
		let (master_control, master_label) = volume_control(
			&document,
			"volume-channel-master",
			&name_row,
			"Master",
			&input_row,
			&label_row,
		);
		let master_value = Rc::new(Cell::new(1.));

		// Create volume control for every channel.
		let volumes: Rc<Vec<_>> = Rc::new(
			(0..channel_count)
				.map(|index| {
					// Create gain node.
					let mut options = GainOptions::new();
					options.channel_count(channel_count);
					let gain = GainNode::new_with_options(&context, &options).unwrap();
					let gain_param = gain.gain();
					gain_param.set_value(0.01);
					channel_splitter
						.connect_with_audio_node_and_output(&gain, index)
						.unwrap();
					gain.connect_with_audio_node_and_output_and_input(&channel_merger, 0, index)
						.unwrap();

					// Create HTML control elements.
					let (control, label) = volume_control(
						&document,
						&format!("volume-channel-{index}"),
						&name_row,
						&format!("Channel {index}"),
						&input_row,
						&label_row,
					);

					// Create callback for channel volume.
					let callback = Closure::<dyn FnMut()>::new({
						let master_control = master_control.clone();
						let master_label = master_label.clone();
						let master_value = Rc::clone(&master_value);
						let control = control.clone();
						let context = context.clone();
						move || {
							let value_string = control.value();
							label.set_text_content(Some(&value_string));
							let mut value = value_string.parse().unwrap();

							if master_value.get() < value {
								master_value.set(value);
								master_control.set_value(&value_string);
								master_label.set_text_content(Some(&value_string));
							}

							value /= 100.;

							if value == 0. {
								let end_time = context.current_time() + 0.1;
								gain_param
									.exponential_ramp_to_value_at_time(0.001, end_time)
									.unwrap();
								gain_param.set_value_at_time(0., end_time).unwrap();
							} else {
								gain_param
									.exponential_ramp_to_value_at_time(
										value,
										context.current_time() + 0.1,
									)
									.unwrap();
							}
						}
					});
					control.set_oninput(Some(callback.as_ref().unchecked_ref()));

					(control, callback)
				})
				.collect(),
		);

		// Setup master control callback.
		let event = Event::new("input").unwrap();
		let master_callback = Closure::<dyn FnMut()>::new({
			let master_control = master_control.clone();
			let volumes = Rc::clone(&volumes);
			move || {
				let value = master_control.value();
				master_value.set(value.parse().unwrap());
				master_label.set_text_content(Some(&value));

				for (control, _) in volumes.iter() {
					control.set_value(&value);
					control.dispatch_event(&event).unwrap();
				}
			}
		});
		master_control.set_oninput(Some(master_callback.as_ref().unchecked_ref()));

		// Setup stop button.
		button.set_onclick(Some(
			Closure::once_into_js({
				let button = button.clone();
				move || {
					button.remove();
					table.remove();
					master_control.set_oninput(None);
					drop(master_callback);

					for (control, callback) in Rc::into_inner(volumes).unwrap() {
						control.set_oninput(None);
						drop(callback);
					}

					context.close().unwrap()
				}
			})
			.as_ref()
			.unchecked_ref(),
		));
		container
			.append_child(&document.create_element("br").unwrap())
			.unwrap();
		button.set_inner_text("Stop");
		container.append_child(&button).unwrap();
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

	/// Create table column for volume control.
	fn volume_control(
		document: &Document,
		id: &str,
		name_row: &HtmlTableRowElement,
		name: &str,
		input_row: &HtmlTableRowElement,
		label_row: &HtmlTableRowElement,
	) -> (HtmlInputElement, HtmlLabelElement) {
		let cell = name_row.insert_cell().unwrap();
		cell.set_text_content(Some(name));
		cell.style().set_property("border", "1px solid").unwrap();
		let label: HtmlLabelElement = document.create_element("label").unwrap().unchecked_into();
		label.set_text_content(Some("1"));
		label.set_html_for(id);
		let cell = label_row.insert_cell().unwrap();
		cell.style()
			.set_property("border-right", "1px solid")
			.unwrap();
		cell.append_child(&label).unwrap();
		let control: HtmlInputElement = document.create_element("input").unwrap().unchecked_into();
		control.set_id(id);
		control.set_value("1");
		let style = control.style();
		// Chrome.
		style
			.set_property("-webkit-writing-mode", "vertical-lr")
			.unwrap();
		// Firefox.
		control.set_attribute("orient", "vertical").unwrap();
		// Safari.
		style
			.set_property("-webkit-appearance", "slider-vertical")
			.unwrap();
		control.set_type("range");
		let cell = input_row.insert_cell().unwrap();
		cell.style()
			.set_property("border-right", "1px solid")
			.unwrap();
		cell.append_child(&control).unwrap();

		(control, label)
	}

	/// Example [`AudioWorkletProcessor`].
	struct ExampleProcessor {
		/// Buffer used to fill outputs.
		buffer: Vec<f32>,
	}

	impl ExampleProcessor {
		/// <https://en.wikipedia.org/wiki/A440_(pitch_standard)>
		const BASE_FREQUENCY: f32 = 440.;
	}

	impl ExtendAudioWorkletProcessor for ExampleProcessor {
		type Data = ();

		fn new(
			_: AudioWorkletProcessor,
			_: Option<Self::Data>,
			_: AudioWorkletNodeOptions,
		) -> Self {
			console::log_1(&"`ExampleProcessor` initialized!".into());
			Self {
				buffer: Vec::with_capacity(128),
			}
		}

		#[allow(
			clippy::as_conversions,
			clippy::cast_possible_truncation,
			clippy::cast_precision_loss,
			clippy::cast_sign_loss
		)]
		fn process(&mut self, _: Array, outputs: Array, _: Object) -> bool {
			/// Transform into an oscillating frequency.
			#[allow(clippy::absolute_paths)]
			const TRANSFORM: f32 = 2. * std::f32::consts::PI;

			let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();

			let sample_rate = global.sample_rate();
			let time = global.current_time() as f32;

			let output = outputs.into_iter().exactly_one().unwrap();
			let output: Array = output.unchecked_into();
			let mut output = output.into_iter();

			let first_channel: Float32Array = output.next().unwrap().unchecked_into();
			let samples = first_channel.length() as usize;
			self.buffer.reserve_exact(samples);

			for index in 0..samples {
				let sample = f32::sin(
					Self::BASE_FREQUENCY * TRANSFORM * (time + index as f32 / sample_rate),
				);

				if let Some(entry) = self.buffer.get_mut(index) {
					*entry = sample;
				} else {
					self.buffer.push(sample);
				}
			}

			first_channel.copy_from(&self.buffer);

			for channel in output {
				let channel: Float32Array = channel.unchecked_into();
				channel.set(&first_channel, 0);
			}

			true
		}
	}
}
