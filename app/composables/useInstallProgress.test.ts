import { beforeEach, describe, expect, it, vi } from 'vitest'

import type { InstallProgressPayload } from '~/composables/useInstallProgress'

const listen = vi.hoisted(() => vi.fn())
vi.mock('@tauri-apps/api/event', () => ({ listen }))

const { useInstallProgress } = await import('~/composables/useInstallProgress')

type Handler = (event: { payload?: InstallProgressPayload }) => void

const ignoreProgress = (): void =>
{
	// Tests that only care about subscription lifecycle.
}

/** Captures the handler Tauri would call, so a test can emit events itself. */
const captureHandler = (unlisten = vi.fn()) =>
{
	let handler: Handler = ignoreProgress

	listen.mockImplementation((_name: string, given: Handler) =>
	{
		handler = given
		return Promise.resolve(unlisten)
	})

	const emit = (payload?: InstallProgressPayload): void =>
	{
		handler({ payload })
	}

	return { emit, unlisten }
}

describe('useInstallProgress', () =>
{
	beforeEach(() =>
	{
		listen.mockReset()
	})

	it('reports progress for the operation it started', async () =>
	{
		const seen: InstallProgressPayload[] = []
		let startedId = ''
		const { emit } = captureHandler()

		await useInstallProgress().trackOperation(
			(payload) => seen.push(payload),
			async (operationId) =>
			{
				startedId = operationId
				emit({ operationId, progress: 42, message: 'Downloaded mod: Sodium' })
				return true
			}
		)

		expect(startedId).not.toBe('')
		expect(seen).toEqual([{ operationId: startedId, progress: 42, message: 'Downloaded mod: Sodium' }])
	})

	// The whole reason the backend tags events with an operation id: a late
	// event from an abandoned install must not move the current dialog's bar.
	it('ignores events belonging to another operation', async () =>
	{
		const seen: InstallProgressPayload[] = []
		const { emit } = captureHandler()

		await useInstallProgress().trackOperation(
			(payload) => seen.push(payload),
			async () =>
			{
				emit({ operationId: 'some-other-install', progress: 99 })
				emit(undefined)
				return true
			}
		)

		expect(seen).toEqual([])
	})

	it('unsubscribes once the operation resolves', async () =>
	{
		const { unlisten } = captureHandler()

		await useInstallProgress().trackOperation(ignoreProgress, async () => true)

		expect(unlisten).toHaveBeenCalledOnce()
	})

	it('unsubscribes when the operation throws', async () =>
	{
		const { unlisten } = captureHandler()
		const failingInstall = async (): Promise<boolean> =>
		{
			throw new Error('install failed')
		}

		await expect(
			useInstallProgress().trackOperation(ignoreProgress, failingInstall)
		).rejects.toThrow('install failed')

		expect(unlisten).toHaveBeenCalledOnce()
	})

	it('gives each operation its own id', async () =>
	{
		captureHandler()
		const ids: string[] = []
		const collect = async (operationId: string): Promise<boolean> =>
		{
			ids.push(operationId)
			return true
		}

		await useInstallProgress().trackOperation(ignoreProgress, collect)
		await useInstallProgress().trackOperation(ignoreProgress, collect)

		expect(ids[0]).not.toBe(ids[1])
	})
})
