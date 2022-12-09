use std::borrow::Cow;
use std::future::Future;
use std::pin::Pin;

use js_sys::Array;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsCast;
use web_sys::{DedicatedWorkerGlobalScope, Worker, WorkerOptions, WorkerType};

use crate::{Close, Error, ScriptUrl, WorkerHandle};

#[must_use = "does nothing unless spawned"]
#[derive(Clone, Debug)]
pub struct WorkerBuilder<'url> {
	pub url: Cow<'url, ScriptUrl>,
	options: Option<WorkerOptions>,
}

impl WorkerBuilder<'_> {
	pub fn new() -> Result<WorkerBuilder<'static>, Error<'static>> {
		Self::new_with_url(crate::default_script_url())
	}

	pub fn new_with_url<'url, URL: Into<Cow<'url, ScriptUrl>>>(
		url: URL,
	) -> Result<WorkerBuilder<'url>, Error<'url>> {
		let url = url.into();

		if url.is_module() && !crate::has_module_support() {
			return Err(Error::NoModuleSupport(url));
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

	pub fn url<'url, URL: Into<Cow<'url, ScriptUrl>>>(
		mut self,
		url: URL,
	) -> Result<WorkerBuilder<'url>, Cow<'url, ScriptUrl>> {
		let url = url.into();

		if url.is_module() && !crate::has_module_support() {
			return Err(url);
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
