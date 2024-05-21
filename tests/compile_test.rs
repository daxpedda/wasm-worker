#![cfg(not(target_family = "wasm"))]

use std::env;
use std::ffi::OsString;

use ui_test::custom_flags::rustfix::RustfixMode;
use ui_test::dependencies::DependencyBuilder;
use ui_test::{Config, OutputConflictHandling};

#[test]
fn test() {
	let mut config = Config {
		output_conflict_handling: OutputConflictHandling::Ignore,
		target: env::var_os("UI_TEST_TARGET").map(|target| target.into_string().unwrap()),
		..Config::rustc("tests/compile-fail")
	};
	let revisioned = config.comment_defaults.base();
	revisioned.set_custom("rustfix", RustfixMode::Disabled);

	let mut dependency_builder = DependencyBuilder::default();

	if let Some(flags) = env::var_os("UI_TEST_RUSTFLAGS").filter(|flags| !flags.is_empty()) {
		add_flags(&mut dependency_builder.program.envs, flags.clone());
		add_flags(&mut config.program.envs, flags);
	}

	if let Some(args) = env::var_os("UI_TEST_ARGS").filter(|flags| !flags.is_empty()) {
		let args = args.into_string().unwrap();

		for arg in args.split_ascii_whitespace() {
			dependency_builder.program.args.push(arg.into());
		}
	}

	revisioned.set_custom("dependencies", dependency_builder);

	ui_test::run_tests(config).unwrap();
}

fn add_flags(envs: &mut Vec<(OsString, Option<OsString>)>, flags: OsString) {
	if let Some((_, current)) = envs.iter_mut().find(|(key, _)| key == "RUSTFLAGS") {
		if let Some(current) = current {
			current.push(" ");
			current.push(flags);
		} else {
			*current = Some(flags);
		}
	} else {
		envs.push((OsString::from("RUSTFLAGS"), Some(flags)));
	}
}
