// composables/useUpdater.ts
import { relaunch } from '@tauri-apps/plugin-process'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { storeToRefs } from 'pinia'
import { readonly } from 'vue'

import { type AppUpdateInfo, useUpdaterStore } from '~/stores/updater'

// Holds the live Update resource handle between checkForUpdates() and
// downloadAndInstall(). Kept outside Pinia state (module-scoped, shared across
// every useUpdater() call site) so Vue's reactivity never wraps the plugin's
// Resource class — Update carries a private field and an rid Vue has no
// business proxying.
let pendingUpdate: Update | null = null

export const useUpdater = () =>
{
	const updaterStore = useUpdaterStore()
	const { $logger: logger } = useNuxtApp()

	// Extract refs from store using storeToRefs
	const {
		updateInfo,
		isChecking,
		isDownloading,
		isInstalling,
		downloadProgress,
		downloadedBytes,
		totalBytes,
		isUpdateDialogVisible
	} = storeToRefs(updaterStore)

	async function closeUpdateResource(update: Update): Promise<void>
	{
		try
		{
			await update.close()
		}
		catch (error)
		{
			logger.warn({ error }, 'Failed to close updater resource')
		}
	}

	async function replacePendingUpdate(update: Update | null): Promise<void>
	{
		const previous = pendingUpdate
		pendingUpdate = update
		if (previous !== null && previous !== update)
		{
			await closeUpdateResource(previous)
		}
	}

	async function clearPendingUpdate(): Promise<void>
	{
		const update = pendingUpdate
		pendingUpdate = null
		if (update !== null)
		{
			await closeUpdateResource(update)
		}
	}

	async function runCheck(): Promise<AppUpdateInfo | null>
	{
		const update = await check()
		await replacePendingUpdate(update)

		if (update === null)
		{
			updateInfo.value = null
			return null
		}

		const info: AppUpdateInfo = {
			version: update.version,
			currentVersion: update.currentVersion,
			date: update.date,
			body: update.body
		}
		updateInfo.value = info
		return info
	}

	const checkForUpdates = async (): Promise<AppUpdateInfo | null> =>
	{
		logger.debug('Manual update check starting')
		isChecking.value = true
		try
		{
			const result = await runCheck()
			logger.info({
				available: result !== null,
				current: result?.currentVersion,
				latest: result?.version
			}, 'Update check completed')
			if (result !== null)
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
		const update = pendingUpdate
		if (update === null)
		{
			throw new Error('No update available')
		}
		// Detach this handle before starting so a concurrent check cannot close a
		// resource that is actively downloading and installing.
		pendingUpdate = null
		try
		{
			isDownloading.value = true
			downloadProgress.value = 0
			downloadedBytes.value = 0
			totalBytes.value = 0

			logger.debug({ version: update.version }, 'Starting update download')

			await update.downloadAndInstall((event) =>
			{
				switch (event.event)
				{
					case 'Started':
						totalBytes.value = event.data.contentLength ?? 0
						break
					case 'Progress':
						downloadedBytes.value += event.data.chunkLength
						downloadProgress.value = totalBytes.value > 0
							? Math.min(100, Math.round((downloadedBytes.value / totalBytes.value) * 100))
							: 0
						break
					case 'Finished':
						downloadProgress.value = 100
						isDownloading.value = false
						isInstalling.value = true
						break
				}
			})

			logger.info('Update installed successfully, relaunching')
			isUpdateDialogVisible.value = false
			await relaunch()
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
			await closeUpdateResource(update)
		}
	}

	const checkForUpdatesOnStartup = async () =>
	{
		try
		{
			const result = await runCheck()
			if (result !== null)
			{
				isUpdateDialogVisible.value = true
			}
		}
		catch (error)
		{
			logger.warn({ error }, 'Startup update check failed (non-critical)')
		}
	}

	const handleUpdateCancel = async () =>
	{
		isUpdateDialogVisible.value = false
		await clearPendingUpdate()
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
		downloadedBytes: readonly(downloadedBytes),
		totalBytes: readonly(totalBytes),
		isUpdateDialogVisible: readonly(isUpdateDialogVisible),
		checkForUpdates,
		downloadAndInstall,
		handleUpdateCancel,
		formatBytes,
		checkForUpdatesOnStartup
	}
}
