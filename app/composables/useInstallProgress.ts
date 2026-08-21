import { listen, type UnlistenFn } from '@tauri-apps/api/event'

/**
 * What the backend attaches to an `install-progress` event.
 *
 * Every field is optional because this crosses the IPC boundary: the payload is
 * whatever arrived, not whatever we expect.
 */
export interface InstallProgressPayload
{
	operationId?: string
	progress?: number
	message?: string
}

/**
 * Subscribes to `install-progress` for the lifetime of a single operation.
 *
 * The backend tags every progress event with the operation it belongs to, and
 * this is what makes that tag useful: the id and the subscription are created
 * and torn down as a pair, so a late event from an abandoned install cannot
 * move the bar on the one now running.
 */
export function useInstallProgress()
{
	/**
	 * Generates an operation id, subscribes, runs `operation` with that id, and
	 * unsubscribes however it ends — including when the subscription itself is
	 * what threw.
	 */
	const trackOperation = async <T>(
		onProgress: (payload: InstallProgressPayload) => void,
		operation: (operationId: string) => Promise<T>
	): Promise<T> =>
	{
		const operationId = globalThis.crypto.randomUUID()
		let unlisten: UnlistenFn | null = null

		try
		{
			unlisten = await listen<InstallProgressPayload | undefined>('install-progress', (event) =>
			{
				// Typed as possibly absent on purpose: this crosses IPC, so the
				// payload is whatever arrived rather than whatever we declared.
				if (event.payload?.operationId !== operationId) return
				onProgress(event.payload)
			})

			return await operation(operationId)
		}
		finally
		{
			if (typeof unlisten === 'function')
			{
				unlisten()
			}
		}
	}

	return { trackOperation }
}
