use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::ops::Deref;
use std::sync::atomic::AtomicU64;

use js_sys::WebAssembly::Global;
use js_sys::{Number, Object, Reflect};
use once_cell::sync::Lazy;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};

use crate::worker::WorkerContext;
#[cfg(feature = "worklet")]
use crate::worklet::WorkletContext;

pub(crate) static SHIM_URL: Lazy<String> = Lazy::new(|| {
	#[wasm_bindgen]
	extern "C" {
		#[wasm_bindgen]
		type Meta;

		#[wasm_bindgen(js_namespace = import, js_name = meta)]
		static META: Meta;

		#[wasm_bindgen(method, getter)]
		fn url(this: &Meta) -> String;
	}

	META.url()
});

#[derive(Debug)]
pub enum Context {
	Worker(WorkerContext),
	#[cfg(feature = "worklet")]
	Worklet(WorkletContext),
}

impl Context {
	pub fn new() -> Option<Self> {
		#[cfg_attr(not(feature = "worklet"), allow(clippy::let_and_return))]
		let result = WorkerContext::new().map(Self::Worker);
		#[cfg(feature = "worklet")]
		let result = result.or_else(|| WorkletContext::new().map(Self::Worklet));

		result
	}

	#[must_use]
	pub fn tls(&self) -> Tls {
		match self {
			Self::Worker(worker) => worker.tls(),
			#[cfg(feature = "worklet")]
			Self::Worklet(worklet) => worklet.tls(),
		}
	}

	#[must_use]
	pub const fn id(&self) -> u64 {
		match self {
			Self::Worker(worker) => worker.id(),
			#[cfg(feature = "worklet")]
			Self::Worklet(worklet) => worklet.id(),
		}
	}
}

#[wasm_bindgen]
extern "C" {
	pub(crate) type Exports;

	#[wasm_bindgen(method, js_name = __wbindgen_thread_destroy)]
	pub(crate) unsafe fn thread_destroy(this: &Exports, tls_base: &Global, stack_alloc: &Global);

	#[wasm_bindgen(method, getter, js_name = __tls_base)]
	pub(crate) fn tls_base(this: &Exports) -> Global;

	#[wasm_bindgen(method, getter, js_name = __stack_alloc)]
	pub(crate) fn stack_alloc(this: &Exports) -> Global;
}

impl Exports {
	thread_local! {
		#[allow(clippy::use_self)]
		static EXPORTS: Lazy<Exports> = Lazy::new(|| wasm_bindgen::exports().unchecked_into());
	}

	pub(crate) fn with<R>(task: impl FnOnce(&Self) -> R) -> R {
		Self::EXPORTS.with(|exports| task(exports.deref()))
	}
}

pub(crate) static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct Tls {
	pub(crate) id: u64,
	tls_base: f64,
	stack_alloc: f64,
}

impl Tls {
	thread_local! {
		static DESCRIPTOR: Lazy<Object> = Lazy::new(|| {
			let descriptor = Object::new();
			let result = Reflect::set(&descriptor, &"value".into(), &"i32".into()).unwrap();
			debug_assert!(result, "expected setting value to be successful");
			descriptor
		});
	}

	pub(crate) fn new(id: u64, tls_base: &Global, stack_alloc: &Global) -> Self {
		let tls_base = Number::unchecked_from_js(tls_base.value()).value_of();
		let stack_alloc = Number::unchecked_from_js(stack_alloc.value()).value_of();

		Self {
			id,
			tls_base,
			stack_alloc,
		}
	}

	#[must_use]
	pub const fn id(&self) -> u64 {
		self.id
	}

	pub(crate) fn tls_base(&self) -> Global {
		Self::DESCRIPTOR.with(|descriptor| Global::new(descriptor, &self.tls_base.into()).unwrap())
	}

	pub(crate) fn stack_alloc(&self) -> Global {
		Self::DESCRIPTOR
			.with(|descriptor| Global::new(descriptor, &self.stack_alloc.into()).unwrap())
	}
}

#[derive(Debug)]
pub enum DestroyError<T>
where
	T: Debug,
{
	Already(Tls),
	Match { handle: T, tls: Tls },
}

impl<T> Display for DestroyError<T>
where
	T: Debug,
{
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Already(_) => write!(formatter, "this worker was already destroyed"),
			Self::Match { .. } => {
				write!(
					formatter,
					"`Tls` value given does not belong to this worker"
				)
			}
		}
	}
}

impl<T> Error for DestroyError<T> where T: Debug {}

pub(crate) static WAIT_ASYNC_SUPPORT: Lazy<bool> = Lazy::new(|| {
	#[wasm_bindgen]
	extern "C" {
		type Atomics;

		#[wasm_bindgen(static_method_of = Atomics, js_name = waitAsync, getter)]
		fn wait_async() -> JsValue;
	}

	!Atomics::wait_async().is_undefined()
});
