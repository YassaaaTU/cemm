import { beforeEach, describe, expect, it, mock } from 'bun:test'

import type { Manifest, UpdateDiff } from '~/types'

const invoke = mock(() => Promise.resolve<unknown>(null))
mock.module('@tauri-apps/api/core', () => ({ invoke }))

/** Silences the composable's own logging while letting a test assert on it. */
const noop = (): void =>
{
	// Nothing to do; the mock only records that it was called.
}

const logger = { error: mock(noop), info: mock(noop), warn: mock(noop), debug: mock(noop) }
Object.assign(globalThis, { useNuxtApp: () => ({ $logger: logger }) })

const { useTauri } = await import('~/composables/useTauri')

const manifest = (over: Partial<Manifest> = {}): Manifest => ({
	mods: [],
	resourcepacks: [],
	shaderpacks: [],
	datapacks: [],
	config_files: [],
	...over
})

const emptyDiff: UpdateDiff = { removed_addons: [], removed_addon_ids: [], updated_addon_ids: [], new_addons: [] }

/**
 * The diff itself is computed in Rust — by the same function the installer uses
 * to decide what to delete — and its behaviour is covered by the tests beside
 * that code in `installer.rs`. What still lives in TypeScript, and so is worth
 * testing here, is the delegation: that the preview asks the backend rather
 * than recomputing, and that a failed lookup cannot be mistaken for "no
 * changes".
 */
describe('getUpdateDiff', () =>
{
	beforeEach(() =>
	{
		invoke.mockReset()
		logger.error.mockReset()
	})

	it('asks the backend rather than computing the diff locally', async () =>
	{
		invoke.mockResolvedValue(emptyDiff)
		const previous = manifest()
		const incoming = manifest()

		await useTauri().getUpdateDiff(previous, incoming)

		expect(invoke).toHaveBeenCalledWith('get_update_diff', { old: previous, new: incoming })
	})

	it('passes a null old manifest through for a first install', async () =>
	{
		invoke.mockResolvedValue(emptyDiff)
		const incoming = manifest()

		await useTauri().getUpdateDiff(null, incoming)

		expect(invoke).toHaveBeenCalledWith('get_update_diff', { old: null, new: incoming })
	})

	it('returns the backend diff unchanged', async () =>
	{
		const backendDiff: UpdateDiff = {
			removed_addons: ['Lithium'],
			removed_addon_ids: [225643],
			updated_addon_ids: [238222],
			new_addons: ['Sodium']
		}
		invoke.mockResolvedValue(backendDiff)

		expect(await useTauri().getUpdateDiff(manifest(), manifest())).toEqual(backendDiff)
	})

	// Null, not an empty diff. An empty diff renders as "nothing will change"
	// beside an Apply button; the caller has to be able to tell the two apart.
	it('returns null when the backend call fails', async () =>
	{
		invoke.mockRejectedValue(new Error('sidecar is not running'))

		expect(await useTauri().getUpdateDiff(manifest(), manifest())).toBeNull()
		expect(logger.error).toHaveBeenCalled()
	})
})
