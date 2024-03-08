self.onmessage = async __web_thread_event => {
	self.onmessage = undefined
	const [
		__web_thread_module,
		__web_thread_memory,
		__web_thread_task,
		__web_thread_message,
	] = __web_thread_event.data

	initSync(__web_thread_module, __web_thread_memory)
	const __web_thread_terminate_index = await __web_thread_worker_entry(__web_thread_task, __web_thread_message)
	const __web_thread_memory_array = new Int32Array(__web_thread_memory.buffer)
	Atomics.store(__web_thread_memory_array, __web_thread_terminate_index, 1)
	Atomics.notify(__web_thread_memory_array, __web_thread_terminate_index)
	Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0)
}
