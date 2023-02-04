use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use js_sys::Array;
use once_cell::sync::Lazy;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use web_sys::{DedicatedWorkerGlobalScope, Worker, WorkerOptions, WorkerType};

use super::{Close, ModuleSupportError, WorkerContext, WorkerHandle};
use crate::ScriptUrl;

#[must_use = "does nothing unless spawned"]
#[derive(Clone, Debug)]
pub struct WorkerBuilder<'url> {
	url: &'url ScriptUrl,
	options: Option<WorkerOptions>,
}

impl WorkerBuilder<'_> {
	pub fn new() -> Result<WorkerBuilder<'static>, ModuleSupportError> {
		Self::new_with_url(crate::default_script_url())
	}

	pub fn new_with_url(url: &ScriptUrl) -> Result<WorkerBuilder<'_>, ModuleSupportError> {
		if url.is_module() && !has_module_support() {
			return Err(ModuleSupportError);
		}

		let mut builder = WorkerBuilder { url, options: None };

		if builder.url.is_module() {
			builder
				.options
				.get_or_insert_with(WorkerOptions::new)
				.type_(WorkerType::Module);
		}

		Ok(builder)
	}

	pub fn url(mut self, url: &ScriptUrl) -> Result<WorkerBuilder<'_>, ModuleSupportError> {
		if url.is_module() && !has_module_support() {
			return Err(ModuleSupportError);
		}

		if url.is_module() {
			self.options
				.get_or_insert_with(WorkerOptions::new)
				.type_(WorkerType::Module);
		}

		Ok(WorkerBuilder {
			url,
			options: self.options,
		})
	}

	pub fn name(mut self, name: &str) -> Self {
		self.options
			.get_or_insert_with(WorkerOptions::new)
			.name(name);
		self
	}

	pub fn spawn<F1, F2>(&self, f: F1) -> WorkerHandle
	where
		F1: 'static + FnOnce(WorkerContext) -> F2 + Send,
		F2: 'static + Future<Output = Close>,
	{
		let work = Task(Box::new(move |context| {
			Box::pin(async move { f(context).await })
		}));
		let task = Box::into_raw(Box::new(work));

		let worker = if let Some(options) = &self.options {
			Worker::new_with_options(&self.url.url, options)
		} else {
			Worker::new(&self.url.url)
		}
		.unwrap_throw();

		let init = Array::of3(
			&wasm_bindgen::module(),
			&wasm_bindgen::memory(),
			&task.into(),
		);

		worker.post_message(&init).unwrap_throw();

		WorkerHandle(worker)
	}
}

#[doc(hidden)]
#[allow(
	clippy::type_complexity,
	missing_debug_implementations,
	unreachable_pub
)]
pub struct Task(
	Box<
		dyn 'static
			+ FnOnce(WorkerContext) -> Pin<Box<dyn 'static + Future<Output = Close>>>
			+ Send,
	>,
);

#[doc(hidden)]
#[allow(unreachable_pub)]
#[wasm_bindgen]
pub async fn __wasm_worker_entry(task: *mut Task) -> bool {
	// Unhooking the message handler has to happen in JS because loading the WASM
	// module will actually yield and introduces a race condition where messages
	// sent will still be handled by the starter message handler.

	// SAFETY: The argument is an address that has to be a valid pointer to a
	// `Task`.
	let Task(work) = *unsafe { Box::from_raw(task) };

	let context = WorkerContext(js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>());
	let close = work(context).await;

	close.to_bool()
}

#[must_use]
fn has_module_support() -> bool {
	static HAS_MODULE_SUPPORT: Lazy<bool> = Lazy::new(|| {
		#[wasm_bindgen]
		struct Tester(Rc<Cell<bool>>);

		#[wasm_bindgen]
		impl Tester {
			#[allow(unreachable_pub)]
			#[wasm_bindgen(getter = type)]
			pub fn type_(&self) {
				self.0.set(true);
			}
		}

		let tester = Rc::new(Cell::new(false));
		let worker_options = WorkerOptions::from(JsValue::from(Tester(Rc::clone(&tester))));
		let worker = Worker::new_with_options("data:,", &worker_options).unwrap_throw();
		worker.terminate();

		tester.get()
	});

	*HAS_MODULE_SUPPORT
}
