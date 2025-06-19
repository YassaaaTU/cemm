// composables/useUpdater.ts
import { relaunch } from '@tauri-apps/plugin-process'
import { check, type Update } from '@tauri-apps/plugin-updater'

export const useUpdater = () =>
{
	const logger = usePinoLogger()

	// Update dialog state
	const isUpdateDialogVisible = useState('update.dialogVisible', () => false)
	const isDownloading = useState('update.downloading', () => false)
	const downloadProgress = useState('update.progress', () => 0)
	const updateInfo = useState<Update | null>('update.info', () => null)
	const updatePhase = useState<'available' | 'downloading' | 'complete'>('update.phase', () => 'available')

	const checkForUpdates = async () =>
	{
		try
		{
			logger.info('Checking for updates...')
			const update = await check()

			if (update != null)
			{
				logger.info('Update available', {
					version: update.version,
					currentVersion: update.currentVersion
				})

				// Show our custom dialog instead of native ask dialog
				updateInfo.value = update
				updatePhase.value = 'available'
				isUpdateDialogVisible.value = true
				return update
			}
			else
			{
				logger.info('No updates available')
				return null
			}
		}
		catch (error)
		{
			logger.error('Failed to check for updates', { error })
			throw error
		}
	}

	const handleUpdateConfirm = async () =>
	{
		if (updateInfo.value === null) return

		try
		{
			logger.info('Starting update download...')

			// Switch to downloading phase
			updatePhase.value = 'downloading'
			isDownloading.value = true
			downloadProgress.value = 0

			// Simulate progress updates (Tauri updater doesn't provide real progress)
			const progressInterval = setInterval(() =>
			{
				if (downloadProgress.value < 90)
				{
					downloadProgress.value += 10
				}
			}, 300)

			// Download and install the update
			await updateInfo.value.downloadAndInstall()

			// Complete the progress
			clearInterval(progressInterval)
			downloadProgress.value = 100

			logger.info('Update downloaded and installed successfully')

			// Switch to complete phase
			updatePhase.value = 'complete'
			isDownloading.value = false

			// Auto-close dialog after showing completion
			setTimeout(() =>
			{
				closeUpdateDialog()
				handleRestart()
			}, 2000)
		}
		catch (error)
		{
			logger.error('Failed to download/install update', { error })
			isDownloading.value = false
			downloadProgress.value = 0
			throw error
		}
	}

	const handleUpdateCancel = () =>
	{
		logger.info('Update cancelled by user')
		closeUpdateDialog()
	}

	const closeUpdateDialog = () =>
	{
		isUpdateDialogVisible.value = false
		isDownloading.value = false
		downloadProgress.value = 0
		updateInfo.value = null
		updatePhase.value = 'available'
	}

	const handleRestart = async () =>
	{
		try
		{
			logger.info('Restarting application...')
			await relaunch()
		}
		catch (error)
		{
			logger.error('Failed to restart application', { error })
		}
	}

	const checkForUpdatesOnStartup = async () =>
	{
		// Check for updates 5 seconds after app startup
		setTimeout(checkForUpdates, 5000)
	}

	// Computed values for the dialog
	const dialogTitle = computed(() =>
	{
		switch (updatePhase.value)
		{
			case 'downloading':
				return 'Downloading Update...'
			case 'complete':
				return 'Update Complete!'
			default:
				return 'Update Available'
		}
	})
	const dialogMessage = computed(() =>
	{
		if (updateInfo.value === null) return ''
		switch (updatePhase.value)
		{
			case 'downloading':
				return 'Please wait while the update is being downloaded and installed.'
			case 'complete':
				return 'Update installed successfully! The application will restart automatically.'
			default:
				return 'A new version of CEMM is available. Would you like to download and install it now?'
		}
	})

	const currentVersion = computed(() =>
	{
		if (updateInfo.value === null) return ''
		return updateInfo.value.currentVersion
	})
	const newVersion = computed(() =>
	{
		if (updateInfo.value === null) return ''
		return updateInfo.value.version
	})

	return {
		// State
		isUpdateDialogVisible: readonly(isUpdateDialogVisible),
		isDownloading: readonly(isDownloading),
		downloadProgress: readonly(downloadProgress),
		updatePhase: readonly(updatePhase),

		// Computed
		dialogTitle,
		dialogMessage,
		currentVersion,
		newVersion,

		// Actions
		checkForUpdates,
		handleUpdateConfirm,
		handleUpdateCancel,
		checkForUpdatesOnStartup
	}
}
