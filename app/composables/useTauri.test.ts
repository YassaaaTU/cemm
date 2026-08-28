import { beforeEach, describe, expect, it, mock } from 'bun:test'

import type { InstallBaseline, Manifest, UpdateDiff } from '~/types'

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

/**
 * The addon table hands this whatever `webSiteURL` the manifest carried, and
 * deliberately does not vet it first — the allowlist behind `open_url` is a
 * security boundary over parsed input, and a second copy in TypeScript would
 * be one to drift from. What is worth pinning here is that the value is passed
 * through untouched, and that a refusal stays a refusal rather than becoming a
 * silent success the caller might retry.
 */
describe('openUrl', () =>
{
	beforeEach(() =>
	{
		invoke.mockReset()
		logger.error.mockReset()
	})

	it('passes the project URL to the backend unmodified', async () =>
	{
		invoke.mockResolvedValue(null)

		await useTauri().openUrl('https://www.curseforge.com/minecraft/texture-packs/faithful-32x')

		expect(invoke).toHaveBeenCalledWith('open_url', {
			url: 'https://www.curseforge.com/minecraft/texture-packs/faithful-32x'
		})
	})

	it('logs and does nothing when the backend refuses the URL', async () =>
	{
		invoke.mockRejectedValue('Refusing to open disallowed host: evil.example.com')

		await useTauri().openUrl('https://evil.example.com/')

		expect(logger.error).toHaveBeenCalled()
	})
})

/**
 * The baseline decides what an install deletes, so the two states this call can
 * report have to stay distinguishable: a pack CEMM has no record of (fresh
 * install, nothing to remove) and a lookup that failed (hold the preview back).
 * Reconciliation itself is Rust's, and is covered beside it in `manifest.rs`.
 */
describe('resolveInstallBaseline', () =>
{
	beforeEach(() =>
	{
		invoke.mockReset()
		logger.error.mockReset()
	})

	it('asks the backend for the pack it was given', async () =>
	{
		const baseline: InstallBaseline = { manifest: manifest(), unmanaged_addon_ids: [42] }
		invoke.mockResolvedValue(baseline)

		const outcome = await useTauri().resolveInstallBaseline('D:/Instances/ATM10')

		expect(invoke).toHaveBeenCalledWith('resolve_install_baseline', {
			modpackPath: 'D:/Instances/ATM10'
		})
		expect(outcome).toEqual({ ok: true, value: baseline })
	})

	// A pack with no records at all, which is not the same thing as a failure.
	it('reports a pack CEMM knows nothing about as a successful null', async () =>
	{
		invoke.mockResolvedValue(null)

		expect(await useTauri().resolveInstallBaseline('D:/Instances/New')).toEqual({
			ok: true,
			value: null
		})
	})

	it('keeps the backend message when the lookup fails', async () =>
	{
		invoke.mockRejectedValue(new Error('The installed cemm-manifest.json is not valid'))

		const outcome = await useTauri().resolveInstallBaseline('D:/Instances/ATM10')

		expect(outcome.ok).toBe(false)
		expect(logger.error).toHaveBeenCalled()
	})
})

/**
 * Custom data packs have no CurseForge project and so cannot be manifest
 * entries; they travel as file content. The wrapper's job is only to hand the
 * pack folder over and keep a failure legible — the scan is Rust's, tested in
 * `lib.rs`.
 */
describe('collectCustomDatapacks', () =>
{
	beforeEach(() =>
	{
		invoke.mockReset()
		logger.error.mockReset()
	})

	it('asks the backend for the pack it was given', async () =>
	{
		invoke.mockResolvedValue([])

		const outcome = await useTauri().collectCustomDatapacks('D:/Instances/ATM10')

		expect(invoke).toHaveBeenCalledWith('collect_custom_datapacks', {
			modpackPath: 'D:/Instances/ATM10'
		})
		expect(outcome).toEqual({ ok: true, value: [] })
	})

	it('reports a failed scan rather than passing off an empty list as a result', async () =>
	{
		invoke.mockRejectedValue(new Error('Config import file limit (5000) exceeded'))

		expect((await useTauri().collectCustomDatapacks('D:/Instances/ATM10')).ok).toBe(false)
		expect(logger.error).toHaveBeenCalled()
	})
})
