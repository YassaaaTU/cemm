// composables/useUpdater.ts
import { invoke } from '@tauri-apps/api/core'
import { storeToRefs } from 'pinia'
import { nextTick, readonly } from 'vue'

import { useAppStore } from '~/stores/app'
import { type UpdateInfo, useUpdaterStore } from '~/stores/updater'

export const useUpdater = () =>
{
	const updaterStore = useUpdaterStore()
	// const { $logger } = useNuxtApp()
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
	const checkForUpdates = async (): Promise<UpdateInfo> =>
	{
		const appRepo = appStore.appRepo
		if (!appRepo)
		{
			throw new Error('App repository not configured')
		}
		console.info('🔍 MANUAL update check starting', { appRepo })
		isChecking.value = true
		try
		{
			const result = await invoke<UpdateInfo>('check_for_updates', { repo: appRepo })
			updateInfo.value = result
			console.info('✅ MANUAL update check completed', {
				available: result.available, current: result.current_version,
				latest: result.latest_version,
				repo: appRepo,
				downloadUrl: result.download_url,
				assetName: result.asset_name
			})
			if (result.available)
			{
				console.info('🎯 MANUAL check: Setting dialog visible = true')
				isUpdateDialogVisible.value = true
				console.info('🎯 MANUAL check: Dialog visible value after setting:', isUpdateDialogVisible.value)
				nextTick(() =>
				{
					console.info('🎯 MANUAL check: Dialog visible value in nextTick:', isUpdateDialogVisible.value)
				})
			}
			else
			{
				console.info('ℹ️ MANUAL check: No update available, dialog stays hidden')
			}
			return result
		}
		catch (error)
		{
			console.error('Update check failed', error)
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
			const progressInterval = setInterval(() =>
			{
				downloadProgress.value = Math.min(downloadProgress.value + 10, 90)
			}, 200)

			console.info('Starting update download', {
				url: info.download_url,
				asset: info.asset_name
			})
			const filePath = await invoke('download_updater_file', {
				downloadUrl: info.download_url,
				assetName: info.asset_name
			}) as string
			clearInterval(progressInterval)
			downloadProgress.value = 100
			isDownloading.value = false
			isInstalling.value = true
			console.info('Starting update installation', {
				filePath,
				size: (info.size != null) ? formatBytes(info.size) : 'unknown'
			})
			await invoke('install_updater_file', { filePath })
			console.info('Update installed successfully')
			isUpdateDialogVisible.value = false
		}
		catch (error)
		{
			console.error('Update installation failed', error)
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
			console.warn('App repository not configured, skipping startup update check')
			return
		}
		try
		{
			console.info('🚀 STARTUP update check starting', { repo: appRepo })
			const result = await invoke<UpdateInfo>('check_for_updates', { repo: appRepo })
			updateInfo.value = result
			console.info('✅ STARTUP update check completed', {
				available: result.available,
				current: result.current_version,
				latest: result.latest_version,
				repo: appRepo
			})
			if (result.available)
			{
				console.info('🎯 STARTUP check: Setting dialog visible = true')
				isUpdateDialogVisible.value = true
			}
			else
			{
				console.info('ℹ️ STARTUP check: No update available, dialog stays hidden')
			}
		}
		catch (error)
		{
			console.warn('Startup update check failed (non-critical)', error)
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
