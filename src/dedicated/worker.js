let __wasm_worker_wasm;

function __wasm_worker_close() {
	__wasm_worker_wasm.__wbindgen_thread_destroy();
	self.close();
}

self.onmessage = async __wasm_worker_event => {
	self.onmessage = undefined;

	const [__wasm_worker_module, __wasm_worker_memory, __wasm_worker_task] = __wasm_worker_event.data;
	__wasm_worker_wasm = await __wasm_worker_wasm_bindgen(__wasm_worker_module, __wasm_worker_memory);

	if (await Promise.resolve(__wasm_worker_entry(__wasm_worker_task)) === true) {
		__wasm_worker_close();
	}
};
