//! TODO
//!
//! Things to note:
//! - Will fail on import when used with the `no-modules` target.
//! - Blocking is not recommended.
//!
//! Browser bugs:
//! - Firefox doesn't support `TextEncoder`/`TextDecoder` in audio worklets: <https://bugzilla.mozilla.org/show_bug.cgi?id=1826432>.
//! - Firefox doesn't support module service workers: <https://bugzilla.mozilla.org/show_bug.cgi?id=1360870>.
//! - Browsers don't support blocking in shared workers:
//!   - Firefox: <https://bugzilla.mozilla.org/show_bug.cgi?id=1359745>
//!   - Safari: ?
//! - Spec doesn't allow cross-origin isolation in shared and service workers: <https://github.com/w3c/ServiceWorker/pull/1545>.
//! - Browsers don't support spawning and blocking afterwards (e.g.
//!   `spawn(..).join()`):
//!   - Chrome:
//!     - Spawning: <https://issues.chromium.org/issues/40633395>
//!     - `postMessage()`: <https://issues.chromium.org/issues/40687798>
//!   - Safari: ?
//! - Browsers don't properly shutdown audio worklet when state is `closed`:
//!   - Chrome: <https://issues.chromium.org/issues/40072701>
//!   - Firefox: <https://bugzilla.mozilla.org/show_bug.cgi?id=1878516>
//!   - Safari: ?

#![cfg_attr(
	all(
		target_family = "wasm",
		target_os = "unknown",
		target_feature = "atomics"
	),
	feature(stdarch_wasm_atomic_wait)
)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
mod thread;
#[cfg(any(all(target_family = "wasm", target_os = "unknown"), docsrs))]
#[cfg_attr(docsrs, doc(cfg(Web)))]
pub mod web;

pub use std::thread::*;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
#[allow(deprecated)]
pub use self::thread::{
	available_parallelism, current, park, park_timeout, park_timeout_ms, scope, sleep, sleep_ms,
	spawn, yield_now, Builder, JoinHandle, Scope, ScopedJoinHandle, Thread, ThreadId,
};

#[cfg(all(
	target_family = "wasm",
	target_os = "unknown",
	target_feature = "exception-handling"
))]
compile_error!("this library does not work correctly with the exception handling proposal");
