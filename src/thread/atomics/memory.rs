//! TLS destruction handling.

use std::cell::OnceCell;

use js_sys::WebAssembly::Global;
use js_sys::{Number, Object};
use wasm_bindgen::JsCast;

use super::js::{Exports, GlobalDescriptor};
#[cfg(feature = "audio-worklet")]
use super::spawn::{Command, THREAD_HANDLER};

/// Holds pointers to the memory of a thread.
#[derive(Debug)]
pub(super) struct ThreadMemory {
	/// TLS memory.
	tls_base: f64,
	/// Stack memory.
	stack_alloc: f64,
}

impl ThreadMemory {
	/// Create new [`ThreadMemory`] for the calling thread.
	pub(super) fn new() -> Self {
		thread_local! {
			static EXISTS: OnceCell<()> = const { OnceCell::new() };
		}

		EXISTS
			.with(|exists| exists.set(()))
			.expect("created `ThreadMemory` twice for this thread");

		let exports: Exports = wasm_bindgen::exports().unchecked_into();
		let tls_base = Number::unchecked_from_js(exports.tls_base().value()).value_of();
		let stack_alloc = Number::unchecked_from_js(exports.stack_alloc().value()).value_of();

		Self {
			tls_base,
			stack_alloc,
		}
	}

	/// Schedules the memory of the referenced thread to be destroyed.
	///
	/// # Safety
	///
	/// The thread is not allowed to be used while or after this function is
	/// executed.
	pub(super) unsafe fn destroy(self) {
		#[cfg(not(feature = "audio-worklet"))]
		{
			debug_assert!(
				super::is_main_thread(),
				"called `ThreadMemory::destroy()` from outside the main thread"
			);
			// SAFETY: Safety has to be uphold by the caller. See `destroy_internal()` for
			// more details.
			unsafe { self.destroy_internal() };
		}

		#[cfg(feature = "audio-worklet")]
		if super::is_main_thread() {
			// SAFETY: Safety has to be uphold by the caller. See `destroy_internal()` for
			// more details.
			unsafe { self.destroy_internal() };
		} else {
			THREAD_HANDLER
				.get()
				.expect("called `ThreadMemory::destroy()` before main thread was initizalized")
				.send(Command::Destroy(self))
				.expect("`Receiver` was somehow dropped from the main thread");
		}
	}

	/// Destroys the memory of the referenced thread.
	///
	/// # Safety
	///
	/// The thread is not allowed to be used while or after this function is
	/// executed.
	unsafe fn destroy_internal(self) {
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

		// SAFETY: This is guaranteed to be called only once for the corresponding
		// thread because `Self::new()` prevents two objects to the same thread from
		// being created and `ThreadMemory::destroy_internal()` consumes itself. Other
		// safety guarantees have to be uphold by the caller.
		EXPORTS.with(|exports| unsafe {
			exports.thread_destroy(&tls_base, &stack_alloc);
		});
	}
}
