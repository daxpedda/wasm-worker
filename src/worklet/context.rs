use once_cell::unsync::OnceCell;
use web_sys::AudioWorkletGlobalScope;

use crate::common::{Exports, Tls};

#[derive(Clone, Debug)]
pub struct WorkletContext {
	context: AudioWorkletGlobalScope,
	id: u64,
}

impl WorkletContext {
	thread_local! {
		#[allow(clippy::use_self)]
		static BACKUP: OnceCell<WorkletContext>  = OnceCell::new();
	}

	pub(super) fn init(context: AudioWorkletGlobalScope, id: u64) -> Self {
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

	#[must_use]
	#[allow(clippy::missing_const_for_fn)]
	pub fn into_raw(self) -> AudioWorkletGlobalScope {
		self.context
	}

	#[must_use]
	pub fn tls(&self) -> Tls {
		Exports::with(|exports| Tls::new(self.id, &exports.tls_base(), &exports.stack_alloc()))
	}

	#[must_use]
	pub const fn id(&self) -> u64 {
		self.id
	}
}
