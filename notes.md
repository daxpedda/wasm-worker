Test with: `WASM_BINDGEN_EXTERNREF = "1"` `WASM_BINDGEN_WEAKREF = "1"`

`cargo --config "target.wasm32-unknown-unknown.runner = 'wasm-server-runner'" run --example basic --all-features`
