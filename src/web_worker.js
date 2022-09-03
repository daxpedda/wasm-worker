self.onmessage = async event => {
    let [module, memory, work] = event.data;

    let wasm = await wasm_bindgen.initWithoutStart(module, memory);
    wasm.__wasm_thread_entry_point(work);
    // TODO: Determine how this is actually used.
    // See https://github.com/rustwasm/wasm-bindgen/discussions/3063.
    //wasm.__wbindgen_thread_destroy();
    self.close();
};
