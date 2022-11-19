//! Handling of [`WorkerMessage`]s sent with
//! [`DedicatedWorkerGlobalScope::post_message()`](web_sys::DedicatedWorkerGlobalScope::post_message).

mod script;

use futures_channel::mpsc::UnboundedSender;
use once_cell::unsync::OnceCell;
pub(crate) use script::WORKER_SCRIPT;

use crate::window::WindowMessage;

thread_local! {
	pub(crate) static WORKER_STATE: WorkerState = WorkerState::new();
}

pub(crate) struct WorkerState {
	sender: OnceCell<UnboundedSender<WindowMessage>>,
}

impl WorkerState {
	fn new() -> Self {
		Self {
			sender: OnceCell::new(),
		}
	}

	pub(crate) fn init(&self, sender: UnboundedSender<WindowMessage>) {
		self.sender.set(sender).expect("already set `sender`")
	}

	pub(crate) fn sender(&self) -> &UnboundedSender<WindowMessage> {
		self.sender.get().expect("`sender` not set")
	}
}
