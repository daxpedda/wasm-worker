# Documentation

`RUSTDOCFLAGS="--crate-version=main --cfg=web_sys_unstable_apis --cfg=docsrs -Ctarget-feature=+atomics,+bulk-memory" RUSTFLAGS=--cfg=web_sys_unstable_apis cargo doc --all-features --no-deps -Z rustdoc-map -Z rustdoc-scrape-examples`

# Run Examples

`CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS=--cfg=web_sys_unstable_apis CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-server-runner cargo run --example testing --all-features`

# Run Tests

`CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS=--cfg=web_sys_unstable_apis CHROMEDRIVER=chromedriver cargo test --all-features`
`CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS=--cfg=web_sys_unstable_apis GECKODRIVER=geckodriver cargo test --all-features -- --include-ignored`
`CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS=--cfg=web_sys_unstable_apis NO_HEADLESS=1 cargo test --all-features`
