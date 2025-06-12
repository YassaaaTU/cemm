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
		manifest.value = newManifest
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
		setPreviousManifest,
		setUpdateInfo
	}
})
