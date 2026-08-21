import { describe, expect, it } from 'vitest'

import { calculateUpdateDiff } from '~/composables/useTauri'
import type { Addon, Manifest } from '~/types'

const addon = (over: Partial<Addon> & Pick<Addon, 'addon_name' | 'addon_project_id'>): Addon => ({
	addon_file_id: 1,
	cdn_download_url: 'https://mediafilez.forgecdn.net/files/1/1/x.jar',
	mod_folder_path: 'mods',
	version: '1.0.0',
	fileNameOnDisk: `${over.addon_name}.jar`,
	...over
})

const manifest = (over: Partial<Manifest> = {}): Manifest => ({
	mods: [],
	resourcepacks: [],
	shaderpacks: [],
	datapacks: [],
	config_files: [],
	...over
})

describe('calculateUpdateDiff', () =>
{
	it('reports an addon that is gone from the new manifest as removed', () =>
	{
		const diff = calculateUpdateDiff(
			manifest({ mods: [addon({ addon_name: 'Jei', addon_project_id: 238222 })] }),
			manifest()
		)

		expect(diff.removed_addons).toEqual(['Jei'])
		expect(diff.updated_addon_ids).toEqual([])
		expect(diff.new_addons).toEqual([])
	})

	it('reports a version change as an update, keyed by project id', () =>
	{
		const diff = calculateUpdateDiff(
			manifest({ mods: [addon({ addon_name: 'Jei', addon_project_id: 238222, version: '1.0.0' })] }),
			manifest({ mods: [addon({ addon_name: 'Jei', addon_project_id: 238222, version: '2.0.0' })] })
		)

		expect(diff.updated_addon_ids).toEqual([238222])
		expect(diff.removed_addons).toEqual([])
	})

	it('matches a renamed addon by project id rather than calling it removed and added', () =>
	{
		const diff = calculateUpdateDiff(
			manifest({ mods: [addon({ addon_name: 'JEI', addon_project_id: 238222, version: '1.0.0' })] }),
			manifest({ mods: [addon({ addon_name: 'Just Enough Items', addon_project_id: 238222, version: '2.0.0' })] })
		)

		expect(diff.updated_addon_ids).toEqual([238222])
		expect(diff.removed_addons).toEqual([])
		expect(diff.new_addons).toEqual([])
	})

	it('treats an addon disabled in the new manifest as removed', () =>
	{
		const diff = calculateUpdateDiff(
			manifest({ mods: [addon({ addon_name: 'Jei', addon_project_id: 238222 })] }),
			manifest({ mods: [addon({ addon_name: 'Jei', addon_project_id: 238222, disabled: true })] })
		)

		expect(diff.removed_addons).toEqual(['Jei'])
	})

	it('never reports addon changes for a config-only update', () =>
	{
		const diff = calculateUpdateDiff(
			manifest({ mods: [addon({ addon_name: 'Jei', addon_project_id: 238222 })] }),
			manifest({ updateType: 'config', mods: [] })
		)

		expect(diff).toEqual({ removed_addons: [], updated_addon_ids: [], new_addons: [] })
	})

	it('lists every enabled addon as new when there is no old manifest', () =>
	{
		const diff = calculateUpdateDiff(
			null,
			manifest({
				mods: [addon({ addon_name: 'Jei', addon_project_id: 1 })],
				shaderpacks: [addon({ addon_name: 'Complementary', addon_project_id: 2 })],
				datapacks: [addon({ addon_name: 'Hidden', addon_project_id: 3, disabled: true })]
			})
		)

		expect(diff.new_addons).toEqual(['Jei', 'Complementary'])
	})
})
