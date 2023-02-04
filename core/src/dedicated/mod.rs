mod builder;

use std::fmt;
use std::fmt::Display;
use std::future::Future;

use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use web_sys::Worker;

pub use self::builder::WorkerBuilder;
use crate::{global_with, Global, Message};

pub fn spawn<F1, F2>(f: F1) -> WorkerHandle
where
	F1: 'static + FnOnce() -> F2 + Send,
	F2: 'static + Future<Output = Close>,
{
	WorkerBuilder::new().unwrap().spawn(f)
}

#[must_use]
pub fn name() -> Option<String> {
	global_with(|global| {
		if let Global::DedicatedWorker(global) = global {
			Some(global.name())
		} else {
			None
		}
	})
}

pub fn terminate() -> ! {
	global_with(|global| {
		if let Global::DedicatedWorker(_) = global {
			__wasm_worker_close();
			unreachable!("continued after terminating");
		} else {
			panic!("called `terminate` not from a worker");
		}
	})
}

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

	pub fn send_message(message: Message) {
		todo!()
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

#[derive(Clone, Copy, Debug)]
pub struct ModuleSupportError;

impl Display for ModuleSupportError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "browser doesn't support worker modules")
	}
}

impl From<ModuleSupportError> for JsValue {
	fn from(value: ModuleSupportError) -> Self {
		value.to_string().into()
	}
}

#[wasm_bindgen]
extern "C" {
	fn __wasm_worker_close();
}
