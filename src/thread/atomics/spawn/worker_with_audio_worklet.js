import {initSync, __web_thread_worker_entry} from '@shim.js'

onmessage = async event => {
	onmessage = undefined
	const [
		module,
		memory,
		[workletLock, workerLock],
		task,
		message,
	] = event.data

	const memoryArray = new Int32Array(memory.buffer)

	Atomics.wait(memoryArray, workletLock, 1)
	Atomics.add(memoryArray, workerLock, 1)

	while (Atomics.load(memoryArray, workletLock) === 1) {
		if (Atomics.sub(memoryArray, workerLock, 1) === 1)
			Atomics.notify(memoryArray, workerLock)

		Atomics.wait(memoryArray, workletLock, 1)
		Atomics.add(memoryArray, workerLock, 1)
	}

	initSync(module, memory)

	if (Atomics.sub(memoryArray, workerLock, 1) === 1)
		Atomics.notify(memoryArray, workerLock)

	const terminateIndex = await __web_thread_worker_entry(task, message)
	Atomics.store(memoryArray, terminateIndex, 1)
	Atomics.notify(memoryArray, terminateIndex)
	Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0)
}
