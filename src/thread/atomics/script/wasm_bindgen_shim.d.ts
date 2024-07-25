// eslint-disable-next-line @typescript-eslint/no-empty-interface
export interface Message {}

export function initSync(options: initSyncOptions): unknown

export interface initSyncOptions {
	module: BufferSource | WebAssembly.Module
	memory?: WebAssembly.Memory
	thread_stack_size?: number | undefined
}

export function __web_thread_worklet_entry(
	task: number,
	message?: Message,
	port?: MessagePort
): void

export function __web_thread_worklet_register(data: number): void

export function __web_thread_worker_entry(task: number, message: Message): Promise<number>
