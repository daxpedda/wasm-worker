#![allow(clippy::redundant_pub_crate)]
#![allow(
	missing_docs,
	clippy::missing_docs_in_private_items,
	clippy::missing_errors_doc,
	clippy::missing_panics_doc
)]

//! TODO:
//! - Note Chrome nested Worker issue: <https://bugs.chromium.org/p/chromium/issues/detail?id=1408115>.
//! - Document that getting the default worker url will fail if using no-modules
//!   and not starting in a document.
//! - Note possible race condition when sending to newly spawned worker not
//!   receiving messages if receiving message handler wasn't setup yet.

mod dedicated;
mod message;

pub use self::dedicated::{
	spawn, spawn_async, Close, ModuleSupportError, TransferError, WorkerBuilder, WorkerContext,
	WorkerHandle, WorkerHandleRef, WorkerUrl, WorkerUrlFormat,
};
pub use self::message::{
	HasSupportFuture, ImageBitmapSupportFuture, Message, MessageError, MessageEvent, MessageIter,
	Messages, RawMessage, RawMessages, SupportError,
};
