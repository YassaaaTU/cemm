import { defineStore } from 'pinia'

import type { Manifest, ManifestUpdateInfo } from '~/types'

export const useManifestStore = defineStore('manifest', () =>
{
	const manifest = ref<Manifest | null>(null)
	const selectedAddons = ref<string[]>([])
	const previousManifest = ref<Manifest | null>(null)
	const updateInfo = ref<ManifestUpdateInfo | null>(null)
	const excludedAddons = ref<string[]>([])

	function setManifest(newManifest: Manifest | null)
	{
		// Store the current manifest as previous before setting new one
		if (manifest.value !== null && newManifest !== null)
		{
			previousManifest.value = manifest.value
		}
		manifest.value = newManifest
		// Clear exclusions when loading a new manifest
		excludedAddons.value = []
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

	function clearExclusions()
	{
		excludedAddons.value = []
	}

	return {
		manifest,
		selectedAddons,
		previousManifest,
		updateInfo,
		excludedAddons,
		setManifest,
		loadInstalledManifest,
		setPreviousManifest,
		setUpdateInfo,
		toggleExclusion,
		isExcluded,
		clearExclusions
	}
})
