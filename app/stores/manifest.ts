import { defineStore } from 'pinia'

import type { Manifest, UpdateInfo } from '~/types'

export const useManifestStore = defineStore('manifest', () =>
{
	const manifest = ref<Manifest | null>(null)
	const selectedAddons = ref<string[]>([])
	const removedAddons = ref<string[]>([])
	const previousManifest = ref<Manifest | null>(null)
	const updateInfo = ref<UpdateInfo | null>(null)
	function setManifest(newManifest: Manifest | null)
	{
		// Store the current manifest as previous before setting new one
		if (manifest.value !== null && newManifest !== null)
		{
			previousManifest.value = manifest.value
		}
		manifest.value = newManifest
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
	return {
		manifest,
		selectedAddons,
		removedAddons,
		previousManifest,
		updateInfo,
		setManifest,
		loadInstalledManifest,
		setPreviousManifest,
		setUpdateInfo
	}
})
