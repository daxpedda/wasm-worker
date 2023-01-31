use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use js_sys::Array;
use once_cell::sync::Lazy;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{DedicatedWorkerGlobalScope, Worker, WorkerOptions, WorkerType};

use super::{Close, Error, WorkerHandle};
use crate::ScriptUrl;

#[must_use = "does nothing unless spawned"]
#[derive(Clone, Debug)]
pub struct WorkerBuilder<'url> {
	url: &'url ScriptUrl,
	options: Option<WorkerOptions>,
}

impl WorkerBuilder<'_> {
	pub fn new() -> Result<WorkerBuilder<'static>, Error> {
		Self::new_with_url(crate::default_script_url())
	}

	pub fn new_with_url(url: &ScriptUrl) -> Result<WorkerBuilder<'_>, Error> {
		if url.is_module() && !has_module_support() {
			return Err(Error::ModuleSupport);
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

	pub fn url(mut self, url: &ScriptUrl) -> Result<WorkerBuilder<'_>, Error> {
		if url.is_module() && !has_module_support() {
			return Err(Error::ModuleSupport);
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
		F1: 'static + FnOnce() -> F2 + Send,
		F2: 'static + Future<Output = Close>,
	{
		let work = Task(Box::new(move || Box::pin(async move { f().await })));
		let task = Box::into_raw(Box::new(work));

		let worker = if let Some(options) = &self.options {
			Worker::new_with_options(self.url.url(), options)
		} else {
			Worker::new(self.url.url())
		}
		.unwrap();

		let init = Array::of3(
			&wasm_bindgen::module(),
			&wasm_bindgen::memory(),
			&task.into(),
		);

		worker.post_message(&init).unwrap();

		WorkerHandle(worker)
	}
}

#[doc(hidden)]
#[allow(missing_debug_implementations, unreachable_pub)]
pub struct Task(
	Box<dyn 'static + FnOnce() -> Pin<Box<dyn 'static + Future<Output = Close>>> + Send>,
);

#[doc(hidden)]
#[allow(unreachable_pub)]
#[wasm_bindgen]
pub async fn __wasm_worker_entry(task: *mut Task) -> bool {
	js_sys::global()
		.unchecked_into::<DedicatedWorkerGlobalScope>()
		.set_onmessage(None);

	// SAFETY: The argument is an address that has to be a valid pointer to a
	// `Task`.
	let Task(work) = *unsafe { Box::from_raw(task) };

	let close = work().await;

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
		let worker = Worker::new_with_options("data:,", &worker_options).unwrap();
		worker.terminate();

		tester.get()
	});

	*HAS_MODULE_SUPPORT
}
