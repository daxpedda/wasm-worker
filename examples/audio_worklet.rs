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
	use std::iter;
	use std::rc::Rc;
	use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
	use std::sync::Arc;

	use itertools::Itertools;
	use js_sys::{Array, Float32Array, Object, Promise, Reflect};
	use wasm_bindgen::closure::Closure;
	use wasm_bindgen::{JsCast, JsValue};
	use wasm_bindgen_futures::JsFuture;
	use web_sys::{
		console, AudioContext, AudioWorkletGlobalScope, AudioWorkletNodeOptions,
		AudioWorkletProcessor, Blob, BlobPropertyBag, Document, HtmlButtonElement, HtmlElement,
		HtmlInputElement, HtmlTableElement, HtmlTableRowElement, Url,
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

		// Get output channel count.
		let channel_count = context.destination().channel_count();

		// Create table to present slider for each channel.
		let table = VolumeControlTable::new(document.clone(), &container);

		// Create master volume control elements.
		let (master_builder, master_mute_callback) = table.volume_control("Master");

		// Create volume control elements for every channel.
		let volumes: Rc<Vec<_>> = Rc::new(
			(0..channel_count)
				.map(|index| {
					// Create control elements.
					let (builder, mute_callback) =
						table.volume_control(&format!("Channel {index}"));

					// Create callback for controlling volume.
					let slider_callback = Closure::<dyn Fn()>::new({
						let master_builder = master_builder.clone();
						let builder = builder.clone();
						move || {
							let value_string = builder.slider.value();
							builder.label.set_inner_text(&value_string);
							let value = value_string.parse().unwrap();
							builder.shared.volume.store(value, Ordering::Relaxed);

							// If the master volume is lower, we increase it, otherwise its weird
							// that master volume is lower then the highest volume.
							if master_builder.shared.volume.load(Ordering::Relaxed) < value {
								master_builder.shared.volume.store(value, Ordering::Relaxed);
								master_builder.slider.set_value(&value_string);
								master_builder.label.set_inner_text(&value_string);
							}
						}
					});
					builder
						.slider
						.set_oninput(Some(slider_callback.as_ref().unchecked_ref()));

					VolumeControl {
						slider: builder.slider,
						_slider_callback: slider_callback,
						label: builder.label,
						_mute_callback: mute_callback,
						shared: builder.shared,
					}
				})
				.collect(),
		);

		// Setup master slider callback.
		let master_slider_callback = Closure::<dyn FnMut()>::new({
			let builder = master_builder.clone();
			let volumes = Rc::clone(&volumes);
			move || {
				let value_string = builder.slider.value();
				builder.label.set_inner_text(&value_string);
				let value = value_string.parse().unwrap();
				builder.shared.volume.store(value, Ordering::Relaxed);

				for VolumeControl {
					slider,
					label,
					shared,
					..
				} in volumes.iter()
				{
					// Update values for all channels (even if we are muted).
					slider.set_value(&value_string);
					label.set_inner_text(&value_string);
					shared.volume.store(value, Ordering::Relaxed);
				}
			}
		});
		master_builder
			.slider
			.set_oninput(Some(master_slider_callback.as_ref().unchecked_ref()));

		// Initialize `ExampleProcessor`.
		let data = Data {
			master: master_builder.shared,
			channels: volumes
				.iter()
				.map(|volume| Arc::clone(&volume.shared))
				.collect(),
		};
		let mut options = AudioWorkletNodeOptions::new();
		options.output_channel_count(&Array::of1(&channel_count.into()));
		let worklet = context
			.audio_worklet_node::<ExampleProcessor>("example", data, Some(options))
			.unwrap();
		worklet
			.connect_with_audio_node(&context.destination())
			.unwrap();

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
		/// Buffer used to calculate each sample.
		samples: Vec<f32>,
		/// Buffer used to adjust output for each channel.
		buffer: Vec<f32>,
		/// Data shared between the window and [`ExampleProcessor`].
		shared: Data,
		/// Current volume of each channel.
		volumes: Vec<f32>,
	}

	impl ExampleProcessor {
		/// <https://en.wikipedia.org/wiki/A440_(pitch_standard)>
		const BASE_FREQUENCY: f32 = 440.;
	}

	impl ExtendAudioWorkletProcessor for ExampleProcessor {
		type Data = Data;

		fn new(
			_: AudioWorkletProcessor,
			data: Option<Self::Data>,
			options: AudioWorkletNodeOptions,
		) -> Self {
			console::log_1(&"`ExampleProcessor` initialized!".into());
			let output_channel_count: Array = Reflect::get(&options, &"outputChannelCount".into())
				.unwrap()
				.unchecked_into();
			#[allow(
				clippy::as_conversions,
				clippy::cast_possible_truncation,
				clippy::cast_sign_loss
			)]
			let channel_count = output_channel_count.get(0).as_f64().unwrap() as usize;
			Self {
				samples: Vec::new(),
				buffer: Vec::new(),
				shared: data.unwrap(),
				volumes: vec![0.01; channel_count],
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

			let master_muted = self.shared.master.mute.load(Ordering::Relaxed);

			// Do nothing if master is muted and all channels have reached zero volume.
			if master_muted && self.volumes.iter().all(|volume| *volume == 0.) {
				return true;
			}

			let global: AudioWorkletGlobalScope = js_sys::global().unchecked_into();

			let sample_rate = global.sample_rate();
			let time = global.current_time() as f32;

			let output = outputs.into_iter().exactly_one().unwrap();
			let output: Array = output.unchecked_into();
			let mut output = output.into_iter();

			let first_channel: Float32Array = output.next().unwrap().unchecked_into();
			let sample_size = first_channel.length() as usize;
			self.samples.reserve_exact(sample_size);
			self.buffer.resize(sample_size, 0.);

			let mut sampled = false;

			for ((current, shared), channel) in self
				.volumes
				.iter_mut()
				.zip(&self.shared.channels)
				.zip(iter::once(first_channel).chain(output.map(JsValue::unchecked_into)))
			{
				// If we are muted always set target volume to zero.
				let target = if master_muted || shared.mute.load(Ordering::Relaxed) {
					0.
				} else {
					f32::from(shared.volume.load(Ordering::Relaxed)) / 1000.
				};

				// If this channels target volume is zero and we reached it do nothing.
				#[allow(clippy::float_cmp)]
				if target == 0. && *current == target {
					continue;
				}

				// Calculate base samples for all channels only if we plan to do actual work.
				if !sampled {
					for index in 0..sample_size {
						let sample = f32::sin(
							Self::BASE_FREQUENCY * TRANSFORM * (time + index as f32 / sample_rate),
						);

						if let Some(entry) = self.samples.get_mut(index) {
							*entry = sample;
						} else {
							self.samples.push(sample);
						}
					}

					sampled = true;
				};

				for (base_sample, out_sample) in self.samples.iter().zip(&mut self.buffer) {
					*out_sample = *base_sample * *current;

					#[allow(clippy::float_cmp)]
					if *current != target {
						if (current.abs() - target.abs()).abs() > 0.0001 {
							if *current < target {
								*current += 0.0001;
							} else {
								*current -= 0.0001;
							}
						} else {
							*current = target;
						}
					}
				}

				channel.copy_from(&self.buffer);
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
		/// Volume value label.
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
		fn volume_control(&self, name: &str) -> (VolumeControlBuilder, Closure<dyn Fn()>) {
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
			let label = self.value.insert_cell().unwrap();
			label
				.style()
				.set_property("border-right", "1px solid")
				.unwrap();
			label.set_inner_text("10");
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

			let shared = Arc::new(SharedData {
				volume: AtomicU8::new(10),
				mute: AtomicBool::new(false),
			});

			// Create callback for mute button.
			let mute_callback = Closure::<dyn Fn()>::new({
				let mute = mute.clone();
				let shared = Arc::clone(&shared);
				move || {
					// If we are muted, unmute.
					if shared.mute.load(Ordering::Relaxed) {
						#[allow(clippy::non_ascii_literal)]
						mute.set_inner_text("🔊");
						shared.mute.store(false, Ordering::Relaxed);
					}
					// If we are not muted, mute.
					else {
						#[allow(clippy::non_ascii_literal)]
						mute.set_inner_text("🔇");
						shared.mute.store(true, Ordering::Relaxed);
					}
				}
			});
			mute.set_onclick(Some(mute_callback.as_ref().unchecked_ref()));

			(
				VolumeControlBuilder {
					slider,
					label,
					shared,
				},
				mute_callback,
			)
		}

		/// Remove the table from the document.
		fn remove(self) {
			self.table.remove();
		}
	}

	/// Elements to build a [`VolumeControl`].
	#[derive(Clone)]
	struct VolumeControlBuilder {
		/// The volume slider.
		slider: HtmlInputElement,
		/// Label showing the current value.
		label: HtmlElement,
		/// Data shared between the window and [`ExampleProcessor`].
		shared: Arc<SharedData>,
	}

	/// Data shared between the window and [`ExampleProcessor`].
	struct SharedData {
		/// Volume for this channel.
		volume: AtomicU8,
		/// If this channel is muted.
		mute: AtomicBool,
	}

	/// Stores volume control elements.
	struct VolumeControl {
		/// The volume slider.
		slider: HtmlInputElement,
		/// Callback handling slider input.
		_slider_callback: Closure<dyn Fn()>,
		/// Label showing the current value.
		label: HtmlElement,
		/// Callback handling mute button.
		_mute_callback: Closure<dyn Fn()>,
		/// Data shared with [`ExampleProcessor`].
		shared: Arc<SharedData>,
	}

	/// Data shared between the window and [`ExampleProcessor`].
	struct Data {
		/// Master shared data..
		master: Arc<SharedData>,
		/// Shared data for each channel.
		channels: Vec<Arc<SharedData>>,
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
