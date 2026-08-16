import { defineStore } from 'pinia'

// Plain display data extracted from the plugin's Update resource. The Update
// object itself is never stored here — see useUpdater.ts.
export interface AppUpdateInfo
{
	version: string
	currentVersion: string
	date?: string
	body?: string
}

export const useUpdaterStore = defineStore('updater', () =>
{
	const updateInfo = ref<AppUpdateInfo | null>(null)
	const isChecking = ref(false)
	const isDownloading = ref(false)
	const isInstalling = ref(false)
	const downloadProgress = ref(0)
	const downloadedBytes = ref(0)
	const totalBytes = ref(0)
	const isUpdateDialogVisible = ref(false)

	return {
		updateInfo,
		isChecking,
		isDownloading,
		isInstalling,
		downloadProgress,
		downloadedBytes,
		totalBytes,
		isUpdateDialogVisible
	}
})
