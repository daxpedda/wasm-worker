#![allow(clippy::redundant_pub_crate)]
#![allow(
	missing_docs,
	clippy::missing_docs_in_private_items,
	clippy::missing_errors_doc,
	clippy::missing_panics_doc
)]

//! TODO:
//! - Note Chrome nested worker issue: <https://bugs.chromium.org/p/chromium/issues/detail?id=1408115>.
//! - Note Firefox nested worker issue: <https://bugzilla.mozilla.org/show_bug.cgi?id=1817152>.
//! - Document that getting the default worker url will fail if using no-modules
//!   and not starting in a document.
//! - Note possible race condition when sending to newly spawned worker not
//!   receiving messages if receiving message handler is setup after a yield
//!   point.
//! - Note Chrome not cleaning up worklets: <https://bugs.chromium.org/p/chromium/issues/detail?id=1298955>.
//! - Note `TextDe/Encoder` polyfill required: <https://github.com/rustwasm/wasm-bindgen/issues/2367>.
//! - Note workaround for missing ports on worklet creation: <https://github.com/WebAudio/web-audio-api/issues/2456>.

mod common;
pub mod dedicated;
mod global;
pub mod message;
pub mod worklet;

pub use dedicated::{spawn, spawn_async};
