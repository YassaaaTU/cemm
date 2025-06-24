import { defineStore } from 'pinia'
import { ref } from 'vue'

export interface UpdateInfo
{
	available: boolean
	current_version: string
	latest_version: string
	download_url?: string
	asset_name?: string
	size?: number
}

export const useUpdaterStore = defineStore('updater', () =>
{
	const updateInfo = ref<UpdateInfo | null>(null)
	const isChecking = ref(false)
	const isDownloading = ref(false)
	const isInstalling = ref(false)
	const downloadProgress = ref(0)
	const isUpdateDialogVisible = ref(false)

	return {
		updateInfo,
		isChecking,
		isDownloading,
		isInstalling,
		downloadProgress,
		isUpdateDialogVisible
	}
})
