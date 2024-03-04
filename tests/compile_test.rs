#![cfg(not(target_family = "wasm"))]

use std::env;
use std::ffi::OsString;

use ui_test::spanned::Spanned;
use ui_test::{Config, Mode, OutputConflictHandling, RustfixMode};

#[test]
fn test() {
	let mut config = Config {
		dependencies_crate_manifest_path: Some("Cargo.toml".into()),
		output_conflict_handling: OutputConflictHandling::Ignore,
		target: env::var_os("UI_TEST_TARGET").map(|target| target.into_string().unwrap()),
		..Config::rustc("tests/compile-fail")
	};
	config.comment_defaults.base().mode = Spanned::dummy(Mode::Fail {
		require_patterns: true,
		rustfix: RustfixMode::Disabled,
	})
	.into();

	if let Some(flags) = env::var_os("UI_TEST_RUSTFLAGS").filter(|flags| !flags.is_empty()) {
		add_flags(&mut config.dependency_builder.envs, flags.clone());
		add_flags(&mut config.program.envs, flags);
	}

	if let Some(args) = env::var_os("UI_TEST_ARGS").filter(|flags| !flags.is_empty()) {
		let args = args.into_string().unwrap();

		for arg in args.split_ascii_whitespace() {
			config.dependency_builder.args.push(arg.into());
		}
	}

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
