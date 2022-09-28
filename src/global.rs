//! Helper to determine the current context.
//!
//! This is important to understand if we are in the window or in a worker and
//! act accordingly. The complexity here is caused by the following
//! requirements:
//! - We want to avoid making any dynamic casts over `js_sys::global()` to
//!   minimize JS callbacks.
//! - Can't directly attempt to cast into the type because the JS shim created
//!   by `wasm-bindgen` would fail to be parsed. This is because `Window` can't
//!   be parsed in a worker context.
//! - To prevent having to do this determination every time we need it, we cache
//!   the result in a thread-local.

use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::DedicatedWorkerGlobalScope;

thread_local! {
	/// Cached [`Global`], so we don't have to run this code multiple times. When accessed the
	/// first time, initialization may panic if the context is not the window or a worker.
	pub(crate) static GLOBAL: Global = Global::new();
}

/// This helps us determine if we are in the window or a worker.
pub(crate) enum Global {
	/// Window.
	Window,
	/// Worker.
	Worker(DedicatedWorkerGlobalScope),
}

impl Global {
	/// Creates a [`Global`].
	// TODO: Clippy false-positive.
	// See <https://github.com/rust-lang/rust-clippy/issues/6902>.
	#[allow(clippy::use_self)]
	fn new() -> Global {
		// We need this to detect the context we are in without getting JS parsing
		// errors from the generated JS shim by `wasm-bindgen`.
		#[wasm_bindgen]
		extern "C" {
			type Global;

			#[wasm_bindgen(method, getter, js_name = Window)]
			fn window(this: &Global) -> JsValue;

			#[wasm_bindgen(method, getter, js_name = DedicatedWorkerGlobalScope)]
			fn worker(this: &Global) -> JsValue;
		}

		let global: Global = js_sys::global().unchecked_into();

		if !global.window().is_undefined() {
			Self::Window
		} else if !global.worker().is_undefined() {
			Self::Worker(global.unchecked_into())
		} else {
			panic!("only supported in a browser or web worker")
		}
	}

	/// Returns the worker global scope. Panics if not in a worker.
	#[cfg(feature = "track")]
	pub(crate) fn worker(&self) -> &DedicatedWorkerGlobalScope {
		match self {
			Self::Window => panic!("expected to be in a worker"),
			Self::Worker(global) => global,
		}
	}
}
