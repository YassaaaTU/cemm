import { defineStore } from 'pinia'

import type { Manifest } from '~/types'

export const useManifestStore = defineStore('manifest', () =>
{
	const manifest = ref<Manifest | null>(null)
	const selectedAddons = ref<string[]>([])
	const removedAddons = ref<string[]>([])

	function setManifest(newManifest: Manifest | null)
	{
		manifest.value = newManifest
	}

	return {
		manifest,
		selectedAddons,
		removedAddons,
		setManifest
	}
})
