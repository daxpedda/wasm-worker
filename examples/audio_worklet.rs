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
	use js_sys::{Array, Float32Array, Object, Promise};
	use wasm_bindgen::closure::Closure;
	use wasm_bindgen::{JsCast, JsValue};
	use wasm_bindgen_futures::JsFuture;
	use web_sys::{
		console, AudioContext, AudioParam, AudioWorkletGlobalScope, AudioWorkletNode,
		AudioWorkletNodeOptions, AudioWorkletProcessor, BaseAudioContext, Blob, BlobPropertyBag,
		ChannelMergerNode, ChannelMergerOptions, ChannelSplitterNode, ChannelSplitterOptions,
		Document, GainNode, GainOptions, HtmlButtonElement, HtmlElement, HtmlInputElement,
		HtmlTableElement, HtmlTableRowElement, Url,
	};
	use web_thread::web::audio_worklet::{
		AudioWorkletGlobalScopeExt, BaseAudioContextExt, ExtendAudioWorkletProcessor,
	};
	use web_thread::web::{self, YieldTime};

	/// `fn main` implementation.
	pub(crate) fn main() {
		console_error_panic_hook::set_once();

		let document = web_sys::window().unwrap().document().unwrap();
		let body = document.body().unwrap();

		// Create a centered container.
		let container = create_centered_container(&document, &body);

		// Create start/stop button.
		let button: HtmlButtonElement = document.create_element("button").unwrap().unchecked_into();
		button.set_inner_text("Start");
		container.append_child(&button).unwrap();

		// Let button start the audio worklet because an [`AudioContext`] can only start
		// after a user-interaction
		button.clone().set_onclick(Some(
			Closure::once_into_js(|| {
				// Disable button during starting.
				button.set_disabled(true);
				button.set_inner_text("Starting ...");
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

	/// Start the example.
	#[allow(clippy::too_many_lines)]
	async fn start(
		document: Document,
		container: HtmlElement,
		start_stop_button: HtmlButtonElement,
	) {
		// Create audio context.
		let context = AudioContext::new().unwrap();

		// Firefox requires a polyfill for `TextDecoder`/`TextEncoder`:
		// <https://bugzilla.mozilla.org/show_bug.cgi?id=1826432>
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

		// Remove start button in preparation of adding new content.
		start_stop_button.remove();

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
		let table = VolumeControlTable::new(document.clone(), &container);

		// Create master volume control elements.
		let (master_slider, master_label, master_mute) = table.volume_control("Master");
		let master_value = Rc::new(Cell::new(10.));
		let master_mute_value = Rc::new(Cell::new(false));

		// Create volume control elements for every channel.
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

					// Create control elements.
					let (slider, label, mute) = table.volume_control(&format!("Channel {index}"));

					// Create callback for controlling volume.
					let slider_value = Rc::new(Cell::new(0.));
					let mute_value = Rc::new(Cell::new(false));
					let slider_callback = Closure::<dyn Fn()>::new({
						let master_slider = master_slider.clone();
						let master_label = master_label.clone();
						let master_value = Rc::clone(&master_value);
						let master_mute_value = Rc::clone(&master_mute_value);
						let slider = slider.clone();
						let label = label.clone();
						let slider_value = Rc::clone(&slider_value);
						let mute_value = Rc::clone(&mute_value);
						let context = context.clone();
						let gain_param = gain_param.clone();
						move || {
							let value_string = slider.value();
							label.set_inner_text(&value_string);
							let value = value_string.parse().unwrap();
							slider_value.set(value);

							// If the master volume is lower, we increase it, otherwise its weird
							// that master volume is lower then the highest volume.
							if master_value.get() < value {
								master_value.set(value);
								master_slider.set_value(&value_string);
								master_label.set_inner_text(&value_string);
							}

							// Only change gain if master and this channel is not muted.
							if !master_mute_value.get() && !mute_value.get() {
								set_gain(&context, &gain_param, value / 1000.);
							}
						}
					});
					slider.set_oninput(Some(slider_callback.as_ref().unchecked_ref()));
					// Create callback for mute button.
					let mute_callback = Closure::<dyn Fn()>::new({
						let master_mute_value = Rc::clone(&master_mute_value);
						let slider_value = Rc::clone(&slider_value);
						let mute = mute.clone();
						let mute_value = Rc::clone(&mute_value);
						let context = context.clone();
						let gain_param = gain_param.clone();
						move || {
							// If we are muted, unmute.
							if mute_value.get() {
								#[allow(clippy::non_ascii_literal)]
								mute.set_inner_text("🔊");
								mute_value.set(false);

								if !master_mute_value.get() {
									set_gain(&context, &gain_param, slider_value.get() / 1000.);
								}
							}
							// If we are not muted, mute.
							else {
								#[allow(clippy::non_ascii_literal)]
								mute.set_inner_text("🔇");
								mute_value.set(true);

								set_gain(&context, &gain_param, 0.);
							}
						}
					});
					mute.set_onclick(Some(mute_callback.as_ref().unchecked_ref()));

					VolumeControl {
						gain_param,
						slider,
						_slider_callback: slider_callback,
						slider_value,
						label,
						_mute_callback: mute_callback,
						mute_value,
					}
				})
				.collect(),
		);

		// Setup master slider callback.
		let master_slider_callback = Closure::<dyn FnMut()>::new({
			let master_slider = master_slider.clone();
			let master_mute_value = Rc::clone(&master_mute_value);
			let volumes = Rc::clone(&volumes);
			let context = context.clone();
			move || {
				let value_string = master_slider.value();
				master_label.set_inner_text(&value_string);
				let value = value_string.parse().unwrap();
				master_value.set(value);

				for VolumeControl {
					gain_param,
					slider,
					slider_value,
					label,
					mute_value,
					..
				} in volumes.iter()
				{
					// Update values for all channels (even if we are muted).
					slider.set_value(&value_string);
					label.set_inner_text(&value_string);
					slider_value.set(value);

					// Only change gain if master and this channel is not muted.
					if !master_mute_value.get() && !mute_value.get() {
						set_gain(&context, gain_param, value / 1000.);
					}
				}
			}
		});
		master_slider.set_oninput(Some(master_slider_callback.as_ref().unchecked_ref()));
		// Setup master mute callback.
		// Create callback for mute button.
		let master_mute_callback = Closure::<dyn Fn()>::new({
			let master_mute = master_mute.clone();
			let master_mute_value = Rc::clone(&master_mute_value);
			let volumes = Rc::clone(&volumes);
			let context = context.clone();
			move || {
				// If we are muted, unmute all channels.
				if master_mute_value.get() {
					#[allow(clippy::non_ascii_literal)]
					master_mute.set_inner_text("🔊");
					master_mute_value.set(false);

					for VolumeControl {
						gain_param,
						slider_value,
						mute_value,
						..
					} in volumes.iter()
					{
						// Only unmute if this channel is not muted.
						if !mute_value.get() {
							set_gain(&context, gain_param, slider_value.get() / 1000.);
						}
					}
				}
				// If we are not muted, mute all channels.
				else {
					#[allow(clippy::non_ascii_literal)]
					master_mute.set_inner_text("🔇");
					master_mute_value.set(true);

					for VolumeControl { gain_param, .. } in volumes.iter() {
						set_gain(&context, gain_param, 0.);
					}
				}
			}
		});
		master_mute.set_onclick(Some(master_mute_callback.as_ref().unchecked_ref()));

		// Setup space before control buttons.
		container
			.append_child(&document.create_element("br").unwrap())
			.unwrap();

		// Setup suspend/resume button.
		let suspend_resume_button: HtmlButtonElement =
			document.create_element("button").unwrap().unchecked_into();
		suspend_resume_button.set_inner_text("Suspend");
		let suspended = Rc::new(Cell::new(false));
		let suspend_resume_callback = Closure::<dyn Fn() -> Promise>::new({
			let button = suspend_resume_button.clone();
			let context = context.clone();
			move || {
				// Disable button during suspending or resuming.
				button.set_disabled(true);

				let button = button.clone();
				let context = context.clone();
				let suspended = Rc::clone(&suspended);
				wasm_bindgen_futures::future_to_promise(async move {
					// If context is suspended, resume.
					if suspended.get() {
						button.set_inner_text("Resuming ...");
						JsFuture::from(context.resume().unwrap()).await.unwrap();
						button.set_inner_text("Suspend");
						suspended.set(false);
					}
					// If context is running, suspend.
					else {
						button.set_inner_text("Suspending ...");
						JsFuture::from(context.suspend().unwrap()).await.unwrap();
						button.set_inner_text("Resume");
						suspended.set(true);
					}

					// Re-enable button after we finished suspending or resuming.
					button.set_disabled(false);

					Ok(JsValue::UNDEFINED)
				})
			}
		});
		suspend_resume_button.set_onclick(Some(suspend_resume_callback.as_ref().unchecked_ref()));
		container.append_child(&suspend_resume_button).unwrap();

		// Setup stop button.
		start_stop_button.set_inner_text("Stop");
		start_stop_button.set_onclick(Some(
			Closure::once_into_js({
				let container = container.clone();
				let start_stop_button = start_stop_button.clone();
				move || {
					// Disable button during stopping.
					start_stop_button.set_disabled(true);
					start_stop_button.set_inner_text("Stopping ...");
					// Disable resume button as well.
					suspend_resume_button.set_disabled(true);
					drop(suspend_resume_callback);

					wasm_bindgen_futures::future_to_promise(async move {
						// Closure audio context.
						JsFuture::from(context.close().unwrap()).await.unwrap();

						// Remove all control elements.
						table.remove();
						suspend_resume_button.remove();
						drop(master_slider_callback);
						drop(master_mute_callback);
						drop(Rc::into_inner(volumes).unwrap());

						// Setup restart button.
						start_stop_button.set_onclick({
							let start_stop_button = start_stop_button.clone();
							Some(
								Closure::once_into_js(move || {
									// Disable button during restarting.
									start_stop_button.set_disabled(true);
									start_stop_button.set_inner_text("Starting ...");
									start_stop_button.set_onclick(None);

									wasm_bindgen_futures::future_to_promise(async {
										start(document, container, start_stop_button).await;
										Ok(JsValue::UNDEFINED)
									})
								})
								.as_ref()
								.unchecked_ref(),
							)
						});
						// Re-enable button after restarting.
						start_stop_button.set_disabled(false);
						start_stop_button.set_inner_text("Start");

						Ok(JsValue::UNDEFINED)
					})
				}
			})
			.as_ref()
			.unchecked_ref(),
		));
		start_stop_button.set_disabled(false);
		container.append_child(&start_stop_button).unwrap();
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
			Self { buffer: Vec::new() }
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

	/// Create centered container by making the body a CSS grid.
	fn create_centered_container(document: &Document, body: &HtmlElement) -> HtmlElement {
		document
			.document_element()
			.unwrap()
			.unchecked_into::<HtmlElement>()
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

		container
	}

	/// Table for all volume control elements.
	struct VolumeControlTable {
		/// Hold [`Document`] to create columns.
		document: Document,
		/// The table itself.
		table: HtmlTableElement,
		/// Name of each channel.
		name: HtmlTableRowElement,
		/// Volume slider.
		slider: HtmlTableRowElement,
		/// Volume value labbel.
		value: HtmlTableRowElement,
		/// Mute button.
		mute: HtmlTableRowElement,
	}

	impl VolumeControlTable {
		/// Creates a new [`VolumeControlTable`].
		fn new(document: Document, container: &HtmlElement) -> Self {
			let table: HtmlTableElement =
				document.create_element("table").unwrap().unchecked_into();
			container.append_child(&table).unwrap();
			let style = table.style();
			style.set_property("border", "1px solid").unwrap();
			style.set_property("border-collapse", "collapse").unwrap();
			let name: HtmlTableRowElement = table.insert_row().unwrap().unchecked_into();
			let slider: HtmlTableRowElement = table.insert_row().unwrap().unchecked_into();
			let value: HtmlTableRowElement = table.insert_row().unwrap().unchecked_into();
			let mute: HtmlTableRowElement = table.insert_row().unwrap().unchecked_into();

			Self {
				document,
				table,
				name,
				slider,
				value,
				mute,
			}
		}

		/// Create table column for volume control elements.
		fn volume_control(&self, name: &str) -> (HtmlInputElement, HtmlElement, HtmlButtonElement) {
			// Name.
			let cell = self.name.insert_cell().unwrap();
			cell.set_inner_text(name);
			cell.style().set_property("border", "1px solid").unwrap();
			// Slider.
			let slider: HtmlInputElement = self
				.document
				.create_element("input")
				.unwrap()
				.unchecked_into();
			slider.set_value("10"); // Default value.
			{
				// Make slider vertical.
				let style = slider.style();
				// Chrome.
				style
					.set_property("-webkit-writing-mode", "vertical-lr")
					.unwrap();
				// Firefox.
				slider.set_attribute("orient", "vertical").unwrap();
				// Safari.
				style
					.set_property("-webkit-appearance", "slider-vertical")
					.unwrap();
			}
			slider.set_type("range");
			let cell = self.slider.insert_cell().unwrap();
			cell.style()
				.set_property("border-right", "1px solid")
				.unwrap();
			cell.append_child(&slider).unwrap();
			// Value label.
			let value = self.value.insert_cell().unwrap();
			value
				.style()
				.set_property("border-right", "1px solid")
				.unwrap();
			value.set_inner_text("10");
			// Mute button.
			let mute: HtmlButtonElement = self
				.document
				.create_element("button")
				.unwrap()
				.unchecked_into();
			#[allow(clippy::non_ascii_literal)]
			mute.set_inner_text("🔊");
			let cell = self.mute.insert_cell().unwrap();
			let style = cell.style();
			style.set_property("border-top", "1px solid").unwrap();
			style.set_property("border-right", "1px solid").unwrap();
			cell.append_child(&mute).unwrap();

			(slider, value, mute)
		}

		/// Remove the table from the document.
		fn remove(self) {
			self.table.remove();
		}
	}

	/// Stores volume control elements.
	struct VolumeControl {
		/// Gain [`AudioParam`] of [`GainNode`].
		gain_param: AudioParam,
		/// The volume slider.
		slider: HtmlInputElement,
		/// Callback handling slider input.
		_slider_callback: Closure<dyn Fn()>,
		/// Stores the value of the slider.
		slider_value: Rc<Cell<f32>>,
		/// Label showing the current value.
		label: HtmlElement,
		/// Callback handling mute button.
		_mute_callback: Closure<dyn Fn()>,
		/// Stores the value of the mute button.
		mute_value: Rc<Cell<bool>>,
	}

	/// Correct way to set gain without causing crackling.
	fn set_gain(context: &BaseAudioContext, param: &AudioParam, value: f32) {
		let end_time = context.current_time() + 0.1;

		// Ramping gain to `0` is not allowed, so we ramp it close to zero and schedule
		// setting to zero then.
		if value == 0. {
			param
				.exponential_ramp_to_value_at_time(0.001, end_time)
				.unwrap();
			param.set_value_at_time(0., end_time).unwrap();
		} else {
			param
				.exponential_ramp_to_value_at_time(value, end_time)
				.unwrap();
		}
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
}
