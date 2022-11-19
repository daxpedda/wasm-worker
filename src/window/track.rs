//! Storing of [`Worker`] objects in a map.
//!
//! This introduces a very small amount of overhead and is therefore hidden
//! behind a crate feature.
//!
//! This was necessary to maintain the following requirements:
//! - Only spawn workers from the window, to prevent parent-child relationships,
//!   to keep in line with [`std::thread`].
//! - Ability to use `Worker.postMessage()` even if the
//!   [`WorkerHandle`](crate::WorkerHandle) is in a worker. This requires the
//!   worker to pass the message through the window, which is the spawner and
//!   therefore the only one capable of holding the actual [`Worker`] object
//!   required to use `Worker.postMessage()`.
//! - Keep the [`WorkerHandle`](crate::WorkerHandle) [`Send`] and [`Sync`]
//!   capable. Holding the [`Worker`] object would prevent that.

use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};

use vec_map::VecMap;
use web_sys::Worker;

/// ID counter for workers.
pub(crate) static IDS: Ids = Ids::new();

/// ID for workers.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Id(usize);

/// [`Id`] counter.
pub(crate) struct Ids(AtomicUsize);

impl Ids {
	/// Creates a new [`Ids`].
	const fn new() -> Self {
		Self(AtomicUsize::new(0))
	}

	/// Returns the next available [`Id`].
	pub(crate) fn next(&self) -> Id {
		// We can stay relaxed because we don't care about the actual ordering, just
		// uniqueness.
		Id(self.0.fetch_add(1, Ordering::Relaxed))
	}
}

/// [`Id`] to [`Worker`] map.
pub(crate) struct Workers(RefCell<VecMap<Worker>>);

impl Workers {
	/// Creates an empty [`Workers`].
	pub(super) fn new() -> Self {
		Self(RefCell::new(VecMap::new()))
	}

	/// Insert a [`Worker`] with the given [`Id`].
	pub(crate) fn push(&self, id: Id, worker: Worker) -> Result<(), Occupied> {
		match self.0.borrow_mut().insert(id.0, worker) {
			Some(_) => Err(Occupied),
			None => Ok(()),
		}
	}

	/// Remove a [`Worker`] with the given [`Id`] from the map and returns it if
	/// it's present.
	pub(crate) fn remove(&self, id: Id) -> Option<Worker> {
		self.0.borrow_mut().remove(id.0)
	}
}

/// Occupied error for [`Workers::push()`].
#[derive(Debug)]
pub(crate) struct Occupied;
