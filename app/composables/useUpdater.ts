// composables/useUpdater.ts
import { invoke } from '@tauri-apps/api/core'
import { storeToRefs } from 'pinia'
import { readonly } from 'vue'

import { useAppStore } from '~/stores/app'
import { type AppUpdateInfo, useUpdaterStore } from '~/stores/updater'

export const useUpdater = () =>
{
	const updaterStore = useUpdaterStore()
	const { $logger: logger } = useNuxtApp()
	const appStore = useAppStore()

	// Extract refs from store using storeToRefs
	const {
		updateInfo,
		isChecking,
		isDownloading,
		isInstalling,
		downloadProgress,
		isUpdateDialogVisible
	} = storeToRefs(updaterStore)
	const checkForUpdates = async (): Promise<AppUpdateInfo> =>
	{
		const appRepo = appStore.appRepo
		if (!appRepo)
		{
			throw new Error('App repository not configured')
		}
		logger.debug({ appRepo }, 'Manual update check starting')
		isChecking.value = true
		try
		{
			const result = await invoke<AppUpdateInfo>('check_for_updates', { repo: appRepo })
			updateInfo.value = result
			logger.info({
				available: result.available,
				current: result.current_version,
				latest: result.latest_version
			}, 'Update check completed')
			if (result.available)
			{
				isUpdateDialogVisible.value = true
			}
			return result
		}
		catch (error)
		{
			logger.error({ error }, 'Update check failed')
			throw error
		}
		finally
		{
			isChecking.value = false
		}
	}

	const downloadAndInstall = async () =>
	{
		const info = updateInfo.value
		if (info === null || !info.available || (info.download_url ?? '').length === 0)
		{
			throw new Error('No update available')
		}
		try
		{
			isDownloading.value = true
			downloadProgress.value = 0

			logger.debug({ url: info.download_url }, 'Starting update download')
			const filePath = await invoke('download_updater_file', {
				downloadUrl: info.download_url,
				assetName: info.asset_name
			}) as string
			downloadProgress.value = 100
			isDownloading.value = false
			isInstalling.value = true
			logger.info({ filePath }, 'Starting update installation')
			await invoke('install_updater_file', { filePath })
			logger.info('Update installed successfully')
			isUpdateDialogVisible.value = false
		}
		catch (error)
		{
			logger.error({ error }, 'Update installation failed')
			isUpdateDialogVisible.value = false
			throw error
		}
		finally
		{
			isDownloading.value = false
			isInstalling.value = false
		}
	}

	const checkForUpdatesOnStartup = async () =>
	{
		const appRepo = appStore.appRepo
		if (!appRepo)
		{
			logger.debug('App repository not configured, skipping startup update check')
			return
		}
		try
		{
			const result = await invoke<AppUpdateInfo>('check_for_updates', { repo: appRepo })
			updateInfo.value = result
			if (result.available)
			{
				isUpdateDialogVisible.value = true
			}
		}
		catch (error)
		{
			logger.warn({ error }, 'Startup update check failed (non-critical)')
		}
	}

	const handleUpdateCancel = () =>
	{
		isUpdateDialogVisible.value = false
	}

	const formatBytes = (bytes: number): string =>
	{
		if (bytes === 0) return '0 Bytes'
		const k = 1024
		const sizes = ['Bytes', 'KB', 'MB', 'GB']
		const i = Math.floor(Math.log(bytes) / Math.log(k))
		return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
	}

	return {
		updateInfo: readonly(updateInfo),
		isChecking: readonly(isChecking),
		isDownloading: readonly(isDownloading),
		isInstalling: readonly(isInstalling),
		downloadProgress: readonly(downloadProgress),
		isUpdateDialogVisible: readonly(isUpdateDialogVisible),
		checkForUpdates,
		downloadAndInstall,
		handleUpdateCancel,
		formatBytes,
		checkForUpdatesOnStartup
	}
}
