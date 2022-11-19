use std::fmt;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::pin::Pin;

use super::Id;

/// Message sent to the window.
#[derive(Debug)]
pub(crate) enum WindowMessage {
	/// Instruct window to spawn a worker.
	Spawn {
		/// ID to use for the spawned worker.
		#[cfg(feature = "track")]
		id: Id,
		/// Task to run.
		task: Task,
	},
	/// Instruct window to terminate a worker.
	#[cfg(feature = "track")]
	Terminate(Id),
	/// Instruct window to delete this [`Worker`][web_sys::Worker] from the
	/// [`Workers`](crate::workers::Workers) list.
	#[cfg(feature = "track")]
	Finished(Id),
}

/// Holds the functions to execute on the worker.
pub(crate) enum Task {
	/// Closure.
	Closure(Box<dyn 'static + FnOnce() + Send>),
	/// [`Future`].
	Future(Box<dyn 'static + FnOnce() -> Pin<Box<dyn 'static + Future<Output = ()>>> + Send>),
}

impl Debug for Task {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Self::Closure(_) => f.debug_struct("Closure").finish(),
			Self::Future(_) => f.debug_struct("Future").finish(),
		}
	}
}
