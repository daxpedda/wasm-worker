self.onmessage = __wasm_worker_event => {
	const [__wasm_worker_module, __wasm_worker_memory, __wasm_worker_data, __wasm_worker_messages] = __wasm_worker_event.data;

	initSync(__wasm_worker_module, __wasm_worker_memory);
	__wasm_worker_worker_entry(__wasm_worker_data, __wasm_worker_messages);
};
