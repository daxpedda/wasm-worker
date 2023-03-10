#![allow(clippy::redundant_pub_crate)]
#![allow(
	missing_docs,
	clippy::missing_docs_in_private_items,
	clippy::missing_errors_doc,
	clippy::missing_panics_doc
)]

//! Notes:
//! - Note Firefox nested worker issue: <https://bugzilla.mozilla.org/show_bug.cgi?id=1817152>.
//! - Document that getting the default worker url will fail if using no-modules
//!   and not starting in a document.
//! - Note Chrome not cleaning up worklets: <https://bugs.chromium.org/p/chromium/issues/detail?id=1298955>.
//! - Note workaround for missing ports on worklet creation: <https://github.com/WebAudio/web-audio-api/issues/2456>.
//! - Note Chrome silently failing on unsupported messages: <https://bugs.chromium.org/p/chromium/issues/detail?id=1341844>.
//!
//! TODO:
//! - Test that all functions (e.g. support checks) also work in workers in
//!   worklets and adjust appropriately.
//! - Implement support for `MessagePort`.
//! - Support sending additional messages that are not transfered.
//! - Implement a higher level implementation of this library.

pub mod common;
mod global;
#[cfg(feature = "message")]
pub mod message;
pub mod worker;
#[cfg(feature = "worklet")]
pub mod worklet;

#[doc(no_inline)]
pub use self::worker::WorkerBuilder;
pub use self::worker::{spawn, spawn_async};
#[doc(no_inline)]
#[cfg(feature = "worklet")]
pub use self::worklet::WorkletBuilder;
#[cfg(feature = "worklet")]
pub use self::worklet::WorkletExt;
