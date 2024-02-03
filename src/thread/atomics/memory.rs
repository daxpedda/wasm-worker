//! TLS destruction handling.

use js_sys::WebAssembly::Global;
use js_sys::{Number, Object};
use wasm_bindgen::JsCast;

use super::js::{Exports, GlobalDescriptor};

/// Holds pointers to the memory of a thread.
pub(super) struct ThreadMemory {
	/// TLS memory.
	tls_base: f64,
	/// Stack memory.
	stack_alloc: f64,
}

impl ThreadMemory {
	/// Create new [`ThreadMemory`] for the calling thread.
	pub(super) fn new() -> Self {
		let exports: Exports = wasm_bindgen::exports().unchecked_into();
		let tls_base = Number::unchecked_from_js(exports.tls_base().value()).value_of();
		let stack_alloc = Number::unchecked_from_js(exports.stack_alloc().value()).value_of();

		Self {
			tls_base,
			stack_alloc,
		}
	}

	/// Destroys the memory of the referenced thread.
	///
	/// # Safety
	///
	/// The thread is not allowed to be used while or after this function is
	/// executed.
	pub(super) unsafe fn destroy(self) {
		thread_local! {
			/// Caches the [`Exports`] object.
			static EXPORTS: Exports = wasm_bindgen::exports().unchecked_into();
			/// Caches the [`GlobalDescriptor`] needed to reconstruct the [`Global`] values.
			static DESCRIPTOR: GlobalDescriptor = {
				let descriptor: GlobalDescriptor = Object::new().unchecked_into();
				descriptor.set_value("i32");
				descriptor
			};
		}

		let (tls_base, stack_alloc) = DESCRIPTOR.with(|descriptor| {
			(
				Global::new(descriptor, &self.tls_base.into())
					.expect("unexpected invalid `Global` constructor"),
				Global::new(descriptor, &self.stack_alloc.into())
					.expect("unexpected invalid `Global` constructor"),
			)
		});

		// SAFETY: User has to uphold the guarantees of this functions documentation.
		EXPORTS.with(|exports| unsafe {
			exports.thread_destroy(&tls_base, &stack_alloc);
		});
	}
}
