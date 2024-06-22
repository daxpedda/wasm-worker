import {initSync, __web_thread_worker_entry} from '@shim.js'

onmessage = async event => {
	onmessage = undefined
	const [module, memory, stackSize, task, message] = event.data

	initSync({module, memory, thread_stack_size: stackSize})
	const terminateIndex = await __web_thread_worker_entry(task, message)
	const memoryArray = new Int32Array(memory.buffer)
	Atomics.store(memoryArray, terminateIndex, 1)
	Atomics.notify(memoryArray, terminateIndex)
	Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0)
}
