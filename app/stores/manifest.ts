import { defineStore } from 'pinia'

import type { Manifest, UpdateInfo } from '~/types'

export const useManifestStore = defineStore('manifest', () =>
{
	const manifest = ref<Manifest | null>(null)
	const selectedAddons = ref<string[]>([])
	const removedAddons = ref<string[]>([])
	const previousManifest = ref<Manifest | null>(null)
	const updateInfo = ref<UpdateInfo | null>(null)
	const excludedAddons = ref<Set<string>>(new Set())

	function setManifest(newManifest: Manifest | null)
	{
		// Store the current manifest as previous before setting new one
		if (manifest.value !== null && newManifest !== null)
		{
			previousManifest.value = manifest.value
		}
		manifest.value = newManifest
		// Clear exclusions when loading a new manifest
		excludedAddons.value = new Set()
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

	function setUpdateInfo(info: UpdateInfo | null)
	{
		updateInfo.value = info
	}

	function toggleExclusion(addonName: string)
	{
		const newSet = new Set(excludedAddons.value)
		if (newSet.has(addonName))
		{
			newSet.delete(addonName)
		}
		else
		{
			newSet.add(addonName)
		}
		excludedAddons.value = newSet
	}

	function isExcluded(addonName: string): boolean
	{
		return excludedAddons.value.has(addonName)
	}

	function clearExclusions()
	{
		excludedAddons.value = new Set()
	}

	return {
		manifest,
		selectedAddons,
		removedAddons,
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
