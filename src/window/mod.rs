//! Window state, includes handling of [`WindowMessage`]s and tracking of
//! workers.

mod message;
#[cfg(feature = "track")]
mod track;

use futures_channel::mpsc::{self, UnboundedSender};
use futures_util::StreamExt;

pub(crate) use self::message::{Task, WindowMessage};
#[cfg(feature = "track")]
use self::track::Workers;
#[cfg(feature = "track")]
pub(crate) use self::track::{Id, IDS};

thread_local! {
	/// State of the window, storing the [sender](UnboundedSender) and the worker [`Ids`](crate::track::Ids).
	pub(crate) static WINDOW_STATE: WindowState = WindowState::new();
}

/// Initiates the message handler and holds the [sender](UnboundedSender) to
/// communicate with the window.
pub(crate) struct WindowState {
	/// Sender to clone for each worker.
	pub(crate) sender: UnboundedSender<WindowMessage>,
	#[cfg(feature = "track")]
	pub(crate) workers: Workers,
}

impl WindowState {
	/// Creates a [`MessageHandler`].
	fn new() -> Self {
		let (sender, mut receiver) = mpsc::unbounded();

		wasm_bindgen_futures::spawn_local(async move {
			while let Some(message) = receiver.next().await {
				match message {
					WindowMessage::Spawn {
						#[cfg(feature = "track")]
						id,
						task,
					} => crate::spawn_from_window(
						#[cfg(feature = "track")]
						id,
						task,
					),
					#[cfg(feature = "track")]
					WindowMessage::Terminate(id) => {
						WINDOW_STATE.with(|state| {
							if let Some(worker) = state.workers.remove(id) {
								worker.terminate();
							}
						});
					}
					#[cfg(feature = "track")]
					WindowMessage::Finished(id) => {
						WINDOW_STATE.with(|state| {
							if state.workers.remove(id).is_none() {
								web_sys::console::warn_1(&"unknown worker ID closed".into());
							}
						});
					}
				}
			}
		});

		Self {
			sender,
			#[cfg(feature = "track")]
			workers: Workers::new(),
		}
	}
}
