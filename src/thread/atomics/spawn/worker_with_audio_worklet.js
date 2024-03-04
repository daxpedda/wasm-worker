self.onmessage = async __web_thread_event => {
	self.onmessage = undefined
	const [
		__web_thread_module,
		__web_thread_memory,
		[__web_thread_worklet_lock, __web_thread_worker_lock],
		__web_thread_task,
		__web_thread_message,
	] = __web_thread_event.data

	const __web_thread_memory_array = new Int32Array(__web_thread_memory.buffer)

	Atomics.wait(__web_thread_memory_array, __web_thread_worklet_lock, 1)
	Atomics.add(__web_thread_memory_array, __web_thread_worker_lock, 1)

	while (Atomics.load(__web_thread_memory_array, __web_thread_worklet_lock) === 1) {
		if (Atomics.sub(__web_thread_memory_array, __web_thread_worker_lock, 1) === 1)
			Atomics.notify(__web_thread_memory_array, __web_thread_worker_lock)

		Atomics.wait(__web_thread_memory_array, __web_thread_worklet_lock, 1)
		Atomics.add(__web_thread_memory_array, __web_thread_worker_lock, 1)
	}

	initSync(__web_thread_module, __web_thread_memory)

	if (Atomics.sub(__web_thread_memory_array, __web_thread_worker_lock, 1) === 1)
		Atomics.notify(__web_thread_memory_array, __web_thread_worker_lock)

	const __web_thread_terminate_index = await __web_thread_worker_entry(__web_thread_task, __web_thread_message)
	Atomics.store(__web_thread_memory_array, __web_thread_terminate_index, 1)
	Atomics.notify(__web_thread_memory_array, __web_thread_terminate_index)
	Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0)
}
