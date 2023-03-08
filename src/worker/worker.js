self.onmessage = async __wasm_worker_event => {
	self.onmessage = undefined;

	const [__wasm_worker_module, __wasm_worker_memory, __wasm_worker_data, __wasm_worker_messages] = __wasm_worker_event.data;

	initSync(__wasm_worker_module, __wasm_worker_memory);
	await Promise.resolve(__wasm_worker_worker_entry(__wasm_worker_data, __wasm_worker_messages));
};
