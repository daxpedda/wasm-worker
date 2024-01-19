//! TODO

#![cfg_attr(
	all(
		target_family = "wasm",
		target_os = "unknown",
		target_feature = "atomics"
	),
	feature(stdsimd)
)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
mod thread;
#[cfg(any(all(target_family = "wasm", target_os = "unknown"), docsrs))]
#[cfg_attr(docsrs, doc(cfg(Web)))]
pub mod web;

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
pub use std::thread::*;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
pub use self::thread::*;

#[cfg(all(
	target_family = "wasm",
	target_os = "unknown",
	target_feature = "exception-handling"
))]
compile_error!("this library does not work correctly with the exception handling proposal");
