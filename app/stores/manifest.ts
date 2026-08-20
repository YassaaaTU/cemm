import { defineStore } from 'pinia'

import type { Manifest, ManifestUpdateInfo } from '~/types'
import { isSameInstance } from '~/utils/instancePath'

/**
 * Addons the instance itself reports as switched off — CurseForge's own
 * enable/disable toggle, or a file renamed to `.disabled` by hand.
 */
function disabledAddonNames(source: Manifest | null): string[]
{
	if (source === null) return []
	return [source.mods, source.resourcepacks, source.shaderpacks, source.datapacks]
		.flat()
		.filter((addon) => addon.disabled === true)
		.map((addon) => addon.addon_name)
}

export const useManifestStore = defineStore('manifest', () =>
{
	const manifest = ref<Manifest | null>(null)
	const previousManifest = ref<Manifest | null>(null)
	const updateInfo = ref<ManifestUpdateInfo | null>(null)
	const excludedAddons = ref<string[]>([])

	/**
	 * Where the loaded manifest came from, and the code it came from — both
	 * belong to the manifest rather than to whichever panel happens to be
	 * mounted. The pack library sets these before navigating, so a manifest
	 * loaded from a card arrives knowing its own provenance instead of the
	 * panel having to rediscover it.
	 *
	 * `sourcePath` is the instance folder this manifest is *about* — the folder
	 * it was read from on the admin side, and the destination its diff baseline
	 * was generated against on the player side. One field for both, because the
	 * question it answers is the same on both: which pack is this manifest for.
	 * `updateCode` is the update reference (player only). Deliberately NOT
	 * persisted: both describe a manifest that only lives for this session.
	 */
	const sourcePath = ref('')
	const updateCode = ref('')

	/**
	 * Whether the loaded manifest is about this pack.
	 *
	 * Both counters read this one store, so "which pack is loaded" has to be
	 * asked of the manifest rather than inferred from the screen. An empty
	 * `sourcePath` means nothing is loaded that could contradict anything, so it
	 * belongs to whoever asks next.
	 */
	function belongsTo(instancePath: string): boolean
	{
		if (sourcePath.value.trim().length === 0) return true
		return isSameInstance(sourcePath.value, instancePath)
	}

	function setManifest(newManifest: Manifest | null)
	{
		// Store the current manifest as previous before setting new one
		if (manifest.value !== null && newManifest !== null)
		{
			previousManifest.value = manifest.value
		}
		manifest.value = newManifest
		// Exclusions belong to the manifest they were made against, so a new one
		// starts from the instance's own state rather than from nothing. An addon
		// CurseForge has switched off is already a decision not to run it;
		// publishing it regardless pushed a mod onto every player that the admin
		// had deliberately turned off. It is still a starting point, not a lock —
		// the row's toggle puts it back in.
		excludedAddons.value = disabledAddonNames(newManifest)
	}

	function loadInstalledManifest(installedManifest: Manifest | null)
	{
		// Set as previous manifest without updating current
		previousManifest.value = installedManifest
	}

	function setPreviousManifest(prev: Manifest | null)
	{
		previousManifest.value = prev
	}

	function setUpdateInfo(info: ManifestUpdateInfo | null)
	{
		updateInfo.value = info
	}

	function toggleExclusion(addonName: string)
	{
		const idx = excludedAddons.value.indexOf(addonName)
		if (idx >= 0)
		{
			excludedAddons.value = excludedAddons.value.filter((n) => n !== addonName)
		}
		else
		{
			excludedAddons.value = [...excludedAddons.value, addonName]
		}
	}

	function isExcluded(addonName: string): boolean
	{
		return excludedAddons.value.includes(addonName)
	}

	/**
	 * Drops the exclusions the admin made by hand and returns to the instance's
	 * own state. It deliberately does NOT re-include addons switched off in
	 * CurseForge: a bulk "include everything" that quietly undid that would
	 * reintroduce the exact thing this floor exists to prevent. Those rows are
	 * put back one at a time, by their own toggle.
	 */
	function clearExclusions()
	{
		excludedAddons.value = disabledAddonNames(manifest.value)
	}

	/**
	 * Full reset. Used when switching between the Install and Publish counters:
	 * both read this same store, so an admin's freshly loaded local instance
	 * left in place would make the player preview diff against the wrong thing.
	 */
	function clearManifest()
	{
		manifest.value = null
		previousManifest.value = null
		updateInfo.value = null
		excludedAddons.value = []
		sourcePath.value = ''
		updateCode.value = ''
	}

	return {
		manifest,
		previousManifest,
		updateInfo,
		excludedAddons,
		sourcePath,
		updateCode,
		belongsTo,
		setManifest,
		loadInstalledManifest,
		setPreviousManifest,
		setUpdateInfo,
		toggleExclusion,
		isExcluded,
		clearExclusions,
		clearManifest
	}
})
