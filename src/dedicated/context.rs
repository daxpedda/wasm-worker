use std::cell::RefCell;
use std::future::Future;

use once_cell::unsync::OnceCell;
use web_sys::DedicatedWorkerGlobalScope;

use crate::common::{Closure, Tls, EXPORTS};
use crate::message::{Message, MessageEvent, SendMessages, TransferError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerContext {
	context: DedicatedWorkerGlobalScope,
	id: usize,
}

impl WorkerContext {
	thread_local! {
		static MESSAGE_HANDLER: RefCell<Option<Closure>> = RefCell::new(None);
		#[allow(clippy::use_self)]
		static BACKUP: OnceCell<WorkerContext> = OnceCell::new();
	}

	pub(super) fn init(context: DedicatedWorkerGlobalScope, id: usize) -> Self {
		let context = Self { context, id };

		Self::BACKUP.with(|once| once.set(context.clone())).unwrap();

		context
	}

	#[must_use]
	pub fn new() -> Option<Self> {
		Self::BACKUP.with(|once| once.get().cloned())
	}

	#[must_use]
	pub const fn as_raw(&self) -> &DedicatedWorkerGlobalScope {
		&self.context
	}

	#[allow(clippy::missing_const_for_fn)]
	#[must_use]
	pub fn into_raw(self) -> DedicatedWorkerGlobalScope {
		self.context
	}

	#[must_use]
	pub fn name(&self) -> Option<String> {
		let name = self.context.name();

		if name.is_empty() {
			None
		} else {
			Some(name)
		}
	}

	#[must_use]
	pub fn has_message_handler(&self) -> bool {
		Self::MESSAGE_HANDLER.with(|message_handler| message_handler.borrow().is_some())
	}

	#[allow(clippy::must_use_candidate)]
	pub fn clear_message_handler(&self) {
		Self::MESSAGE_HANDLER.with(|message_handler| message_handler.borrow_mut().take());
		self.context.set_onmessage(None);
	}

	pub fn set_message_handler<F>(&self, mut new_message_handler: F)
	where
		F: 'static + FnMut(&Self, MessageEvent),
	{
		Self::MESSAGE_HANDLER.with(|message_handler| {
			let mut message_handler = message_handler.borrow_mut();

			let context = self.clone();
			let message_handler = message_handler.insert(Closure::classic(move |event| {
				new_message_handler(&context, MessageEvent::new(event));
			}));

			self.context.set_onmessage(Some(message_handler));
		});
	}

	pub fn set_message_handler_async<F1, F2>(&self, mut new_message_handler: F1)
	where
		F1: 'static + FnMut(&Self, MessageEvent) -> F2,
		F2: 'static + Future<Output = ()>,
	{
		Self::MESSAGE_HANDLER.with(|message_handler| {
			let mut message_handler = message_handler.borrow_mut();

			let context = self.clone();
			let message_handler = message_handler.insert(Closure::future(move |event| {
				new_message_handler(&context, MessageEvent::new(event))
			}));

			self.context.set_onmessage(Some(message_handler));
		});
	}

	pub fn transfer_messages<M, I>(&self, messages: M) -> Result<(), TransferError>
	where
		M: IntoIterator<Item = I>,
		I: Into<Message>,
	{
		self.context.transfer_messages(messages)
	}

	#[must_use]
	pub fn tls(&self) -> Tls {
		EXPORTS.with(|exports| Tls::new(self.id, &exports.tls_base(), &exports.stack_alloc()))
	}

	pub fn close(self) {
		self.context.close();
	}
}
