use once_cell::unsync::OnceCell;
use web_sys::AudioWorkletGlobalScope;

use crate::common::{Tls, EXPORTS};

#[derive(Clone, Debug)]
pub struct WorkletContext {
	context: AudioWorkletGlobalScope,
	id: usize,
}

impl WorkletContext {
	thread_local! {
		#[allow(clippy::use_self)]
		static BACKUP: OnceCell<WorkletContext>  = OnceCell::new();
	}

	pub(super) fn init(context: AudioWorkletGlobalScope, id: usize) -> Self {
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
