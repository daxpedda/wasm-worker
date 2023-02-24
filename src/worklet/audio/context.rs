use once_cell::unsync::OnceCell;
use wasm_bindgen::JsCast;
use web_sys::{AudioWorkletGlobalScope, WorkletGlobalScope};

use crate::common::{Tls, EXPORTS};

#[derive(Clone, Debug)]
pub struct AudioWorkletContext {
	context: AudioWorkletGlobalScope,
	id: usize,
}

impl AudioWorkletContext {
	thread_local! {
		#[allow(clippy::use_self)]
		static BACKUP: OnceCell<AudioWorkletContext>  = OnceCell::new();
	}

	pub(super) fn init(global: WorkletGlobalScope, id: usize) -> Self {
		let context = global.unchecked_into();
		let context = Self { context, id };

		Self::BACKUP.with(|once| once.set(context.clone())).unwrap();

		context
	}

	#[must_use]
	pub fn new() -> Option<Self> {
		Self::BACKUP.with(|once| once.get().cloned())
	}

	#[must_use]
	pub const fn as_raw(&self) -> &AudioWorkletGlobalScope {
		&self.context
	}

	#[allow(clippy::missing_const_for_fn)]
	#[must_use]
	pub fn into_raw(self) -> AudioWorkletGlobalScope {
		self.context
	}

	#[must_use]
	pub fn tls(&self) -> Tls {
		EXPORTS.with(|exports| Tls::new(self.id, &exports.tls_base(), &exports.stack_alloc()))
	}
}
