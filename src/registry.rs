use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};

use vec_map::VecMap;
use web_sys::Worker;

pub(crate) static ID_GENERATOR: IdGenerator = IdGenerator::init();

pub(crate) struct Registry(RefCell<VecMap<Worker>>);

pub(crate) struct IdGenerator(AtomicUsize);

#[derive(Clone, Copy)]
pub(crate) struct Id(usize);

impl Registry {
	fn new() -> Self {
		Self(RefCell::new(VecMap::new()))
	}
}

impl IdGenerator {
	const fn init() -> Self {
		Self(AtomicUsize::new(0))
	}

	pub(crate) fn next(&self) -> Id {
		Id(self.0.fetch_add(1, Ordering::Relaxed))
	}
}
