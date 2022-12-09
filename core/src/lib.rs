#![allow(
	missing_docs,
	clippy::missing_docs_in_private_items,
	clippy::missing_errors_doc,
	clippy::missing_panics_doc
)]

mod builder;
mod global;
mod script_url;

use std::borrow::Cow;
use std::cell::Cell;
use std::fmt::{self, Display};
use std::future::Future;
use std::rc::Rc;

use once_cell::sync::Lazy;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use web_sys::{Worker, WorkerOptions};

pub use self::builder::WorkerBuilder;
pub use self::global::{global_with, Global};
pub use self::script_url::{default_script_url, ScriptUrl};

#[derive(Debug)]
pub struct WorkerHandle(Worker);

impl WorkerHandle {
	#[must_use]
	pub const fn raw(&self) -> &Worker {
		&self.0
	}

	#[allow(clippy::missing_const_for_fn)]
	#[must_use]
	pub fn into_raw(self) -> Worker {
		self.0
	}

	pub fn terminate(self) {
		self.0.terminate();
	}
}

#[derive(Clone, Copy, Debug)]
pub enum Close {
	Yes,
	No,
}

impl Close {
	const fn to_bool(self) -> bool {
		match self {
			Self::Yes => true,
			Self::No => false,
		}
	}
}

pub fn spawn<F1, F2>(f: F1) -> WorkerHandle
where
	F1: 'static + FnOnce() -> F2 + Send,
	F2: 'static + Future<Output = Close>,
{
	WorkerBuilder::new().unwrap().spawn(f)
}

#[must_use]
pub fn has_module_support() -> bool {
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

#[must_use]
pub fn name() -> Option<String> {
	global_with(|global| match global {
		Global::Window(_) => None,
		Global::DedicatedWorker(global) => Some(global.name()),
	})
}

pub fn terminate() {
	__wasm_worker_close();
}

#[derive(Clone, Debug)]
pub enum Error<'url> {
	NoModuleSupport(Cow<'url, ScriptUrl>),
}

impl Display for Error<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::NoModuleSupport(_) => {
				write!(f, "browser doesn't support worker modules")
			}
		}
	}
}

impl From<Error<'_>> for JsValue {
	fn from(value: Error<'_>) -> Self {
		value.to_string().into()
	}
}

#[wasm_bindgen]
extern "C" {
	fn __wasm_worker_close();

	/// JS `try catch` block.
	#[doc(hidden)]
	#[allow(unused_doc_comments)]
	pub fn __wasm_worker_try(fn_: &mut dyn FnMut()) -> JsValue;
}
