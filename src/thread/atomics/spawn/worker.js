self.onmessage = async event => {
	self.onmessage = undefined
	const [module, memory, task, message] = event.data

	initSync(module, memory)
	const terminateIndex = await __web_thread_worker_entry(task, message)
	const memoryArray = new Int32Array(memory.buffer)
	Atomics.store(memoryArray, terminateIndex, 1)
	Atomics.notify(memoryArray, terminateIndex)
	Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0)
}
