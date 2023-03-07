globalThis.registerProcessor('__wasm_worker_InitWasm', class __wasm_worker_InitWasm extends AudioWorkletProcessor {
	constructor(__wasm_worker_options) {
		super();

		const [__wasm_worker_module, __wasm_worker_memory, __wasm_worker_data] = __wasm_worker_options.processorOptions;

		initSync(__wasm_worker_module, __wasm_worker_memory);
		__wasm_worker_worklet_entry(this, __wasm_worker_data);
	}

	process() {
		return false;
	}
});
