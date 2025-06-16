<template>
  <div
    role="main"
    aria-labelledby="user-mode-title"
  >
    <h2
      id="user-mode-title"
      class="text-2xl font-bold mb-4"
    >
      User Mode
    </h2>

    <!-- Quick GitHub Repo Input (top, for user convenience) -->
    <section class="mb-4">
      <fieldset class="fieldset">
        <legend class="fieldset-legend">
          GitHub Repo
        </legend>
        <div class="flex gap-2 items-center">
          <input
            v-model="githubRepo"
            type="text"
            class="input input-bordered flex-1"
            placeholder="user/repo (e.g., john/my-modpack-updates)"
            autocomplete="off"
            aria-label="GitHub repository name"
            @blur="saveGithubRepo"
          />
        </div>
        <p class="label">
          Required for downloading updates from GitHub. This is usually provided by the modpack developer.
        </p>
      </fieldset>
    </section>

    <!-- File Selection Section -->
    <section
      aria-labelledby="file-section"
      class="mb-6"
    >
      <label
        id="file-help"
        for="file-selector"
        class="w-full label label-text"
      >
        Select the modpack directory where you want to install the update:
      </label>
      <file-selector />
    </section>

    <!-- UUID Input Section -->
    <section
      aria-labelledby="uuid-section"
      class="mb-6 w-full flex flex-col gap-2"
    >
      <label
        id="uuid-help"
        for="uuid-input"
        class="w-full label label-text"
      >
        Enter the UUID of the update you want to download from GitHub:
      </label>
      <div class="join w-full">
        <button
          class="btn btn-accent join-item"
          :disabled="downloading || uuid.trim().length === 0 || (path == '' || undefined)"
          :aria-describedby="getDownloadButtonDescription()"
          @click="downloadFromGithub"
        >
          <span
            v-if="!downloading"
            class="flex gap-2 items-center justify-between"
          >
            <Icon
              name="mdi:download"
              class="mr-2"
            />
            Download Manifest
          </span>
          <loading-spinner
            v-else
            :loading="true"
            size="sm"
            message=""
            aria-label="Downloading manifest from GitHub"
          />
        </button>

        <input
          id="uuid-input"
          v-model="uuid"
          type="text"
          class="input input-bordered w-full join-item"
          placeholder="Paste update UUID here"
          aria-describedby="uuid-help"
          :aria-invalid="uuid.trim().length > 0 && uuid.trim().length < 8"
        />
      </div>
      <span class="text-sm text-gray-500">This is usually sent to you by the modpack developer.</span>
    </section>

    <!-- Action Buttons Section -->
    <section
      aria-labelledby="actions-section"
      class="mb-6"
    >
      <div class="flex flex-wrap gap-2">
        <button
          class="btn btn-success"
          :disabled="!canInstall"
          :aria-describedby="getInstallButtonDescription()"
          @click="showPreview = true"
        >
          <span v-if="!installing">Install Update</span>
          <loading-spinner
            v-else
            :loading="true"
            size="sm"
            message=""
            aria-label="Installing modpack update"
          />
        </button>
      </div>
    </section>
    <!-- Error Handling -->
    <app-alert
      v-if="errorState.error"
      class="mt-4"
      :error-state="errorState"
      :retry-operation="retryCurrentOperation"
      :show-technical-details="true"
      @retry="retryCurrentOperation"
      @close="clearError"
    />
    <!-- Simple status messages (for non-error messages) -->
    <app-alert
      v-else-if="statusMessage"
      class="mt-4"
      :message="statusMessage"
      :type="statusType"
    />
    <div
      v-if="manifest"
      class="mt-4"
    >
      <addon-list
        :addons="manifest.mods"
        title="Mods"
        category="mods"
        class="mb-4"
      />
      <addon-list
        v-if="manifest.resourcepacks.length > 0"
        :addons="manifest.resourcepacks"
        title="Resource Packs"
        category="resourcepacks"
        class="mb-4"
      />
      <addon-list
        v-if="manifest.shaderpacks.length > 0"
        :addons="manifest.shaderpacks"
        title="Shader Packs"
        category="shaderpacks"
        class="mb-4"
      />
      <addon-list
        v-if="manifest.datapacks.length > 0"
        :addons="manifest.datapacks"
        title="Data Packs"
        category="datapacks"
        class="mb-4"
      />
    </div>
    <div class="mt-6 flex flex-col gap-2">
      <progress-bar
        :progress="progress"
        :label="getProgressLabel()"
        :color="getProgressColor()"
        :show-percentage="downloading || installing"
      />
    </div>
    <!-- Update Preview Modal -->
    <update-preview
      v-if="showPreview && previewData"
      :preview="previewData"
      :installing="installing"
      :config-files-downloaded="configFilesDownloaded"
      @close="showPreview = false"
      @confirm="confirmInstall"
    />
  </div>
</template>

<script setup lang="ts">
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import type { ConfigFileWithContent, Manifest } from '~/types'

interface InstallProgressEvent
{
	payload?: {
		progress?: number
		message?: string
	}
}

const uuid = ref('')
const progress = ref(0)
const statusMessage = ref('')
const statusType = ref<'success' | 'error' | 'info' | 'warning'>('info')
const { writeFile, readFile, parseMinecraftInstance, installUpdateOptimized } = useTauri()
const manifestStore = useManifestStore()
const manifest = computed(() => manifestStore.manifest)
const appStore = useAppStore()
const downloading = ref(false)
const installing = ref(false)
const downloadedConfigFiles = ref<ConfigFileWithContent[]>([])
const logger = usePinoLogger()
const path = computed(() => appStore.modpackPath)

// GitHub repository reactive property with computed binding to app store
const githubRepo = computed({
	get: () => appStore.githubRepo,
	set: (val: string) =>
	{
		appStore.githubRepo = val
	}
})

// Save GitHub repository function
const saveGithubRepo = () =>
{
	if (githubRepo.value.trim())
	{
		logger.info('GitHub repository saved', { repo: githubRepo.value })
	}
}

// Enhanced error handling
const errorHandler = createErrorHandler(statusMessage, statusType, logger)
const { errorState, handleError, retry, clearError, executeWithRecovery } = errorHandler

// Track current operation for retry functionality
const currentOperation = ref<(() => Promise<void>) | null>(null)
const retryCurrentOperation = async () =>
{
	if (currentOperation.value !== null)
	{
		await retry(currentOperation.value)
	}
}

const canInstall = computed(() =>
	manifest.value !== null
	&& appStore.modpackPath.trim().length > 0
	&& !installing.value
	&& !downloading.value
)

const showPreview = ref(false)
const configFilesDownloaded = ref(false)

// Create preview data for the new UpdatePreview component
const previewData = computed(() =>
{
	if (manifest.value === null) return null

	const oldManifest = manifestStore.previousManifest
	const newManifest = manifest.value

	// Calculate diff using our new helper function
	const diff = {
		removed_addons: [] as string[],
		updated_addons: [] as Array<[string, string]>,
		new_addons: [] as string[]
	}

	if (oldManifest !== null)
	{
		// Helper function to process addon categories
		const processCategory = (
			oldAddons: Array<{ addon_project_id: number, addon_name: string, version: string, disabled?: boolean }>,
			newAddons: Array<{ addon_project_id: number, addon_name: string, version: string, disabled?: boolean }>
		) =>
		{
			// Find removed addons - only count as removed if:
			// 1. Old addon was enabled (not disabled) AND new addon is missing or disabled
			// 2. This prevents showing "will remove" for addons that were already disabled/removed
			for (const oldAddon of oldAddons)
			{
				// Skip if old addon was already disabled - can't "remove" something that wasn't active
				if (oldAddon.disabled === true) continue

				const newAddon = newAddons.find((a) => a.addon_project_id === oldAddon.addon_project_id)
				if (newAddon === undefined || newAddon.disabled === true)
				{
					diff.removed_addons.push(oldAddon.addon_name)
				}
			}

			// Find updated addons (same project ID, different version, not disabled)
			for (const oldAddon of oldAddons)
			{
				const newAddon = newAddons.find((a) => a.addon_project_id === oldAddon.addon_project_id)
				if (newAddon !== undefined && newAddon.disabled !== true && oldAddon.version !== newAddon.version)
				{
					diff.updated_addons.push([oldAddon.version, newAddon.version])
				}
			}

			// Find new addons (in new but not old, and not disabled)
			for (const newAddon of newAddons)
			{
				if (newAddon.disabled === true) continue // Skip disabled addons
				const found = oldAddons.find((a) => a.addon_project_id === newAddon.addon_project_id)
				if (found === undefined)
				{
					diff.new_addons.push(newAddon.addon_name)
				}
			}
		}

		// Process each category
		processCategory(oldManifest.mods, newManifest.mods)
		processCategory(oldManifest.resourcepacks, newManifest.resourcepacks)
		processCategory(oldManifest.shaderpacks, newManifest.shaderpacks)
		processCategory(oldManifest.datapacks, newManifest.datapacks)
	}
	else
	{
		// Fresh install - everything is new (except disabled addons)
		diff.new_addons = [
			...newManifest.mods.filter((addon) => addon.disabled !== true).map((addon) => addon.addon_name),
			...newManifest.resourcepacks.filter((addon) => addon.disabled !== true).map((addon) => addon.addon_name),
			...newManifest.shaderpacks.filter((addon) => addon.disabled !== true).map((addon) => addon.addon_name),
			...newManifest.datapacks.filter((addon) => addon.disabled !== true).map((addon) => addon.addon_name)
		]
	}

	const hasChanges = oldManifest !== null && (
		diff.removed_addons.length > 0
		|| diff.updated_addons.length > 0
		|| diff.new_addons.length > 0
	)
	return {
		oldManifest,
		newManifest,
		diff,
		hasChanges: oldManifest === null ? false : hasChanges,
		configFiles: downloadedConfigFiles.value
	}
})

async function confirmInstall()
{
	showPreview.value = false	// Download config files if not already downloaded and if manifest has config files
	if (!configFilesDownloaded.value && uuid.value.trim().length > 0 && manifest.value !== null && manifest.value.config_files.length > 0)
	{
		try
		{
			await downloadConfigFiles()
		}
		catch (error)
		{
			statusMessage.value = `Failed to download config files: ${error instanceof Error ? error.message : 'Unknown error'}`
			statusType.value = 'error'
			return // Don't proceed with installation if config download fails
		}
	}
	installUpdate()
}

async function downloadFromGithub()
{
	if (uuid.value.trim().length === 0)
	{
		handleError('INVALID_UUID', 'downloadFromGithub')
		return
	}

	const repo = appStore.githubRepo
	if (repo.trim().length === 0)
	{
		handleError('MISSING_GITHUB_SETTINGS', 'downloadFromGithub')
		return
	}

	// Set current operation for retry functionality
	currentOperation.value = downloadFromGithub

	clearError()
	statusType.value = 'info'
	progress.value = 0
	downloading.value = true

	try
	{
		// Debug logging
		logger.info('Starting manifest download', {
			repo,
			uuid: uuid.value.trim(),
			repoLength: repo.length,
			uuidLength: uuid.value.trim().length
		})

		// Phase 1: Download only the manifest with network retry
		progress.value = 10
		statusMessage.value = 'Downloading manifest...'

		const { downloadManifest } = useGithubApi()

		const manifest = await withNetworkRetry(
			async () => await downloadManifest({
				repo,
				uuid: uuid.value.trim(),
				onProgress: (p, msg) =>
				{
					progress.value = Math.min(p / 2, 50) // First half of progress
					if (typeof msg === 'string' && msg.length > 0)
					{
						statusMessage.value = msg
					}
				}
			}),
			3, // maxRetries
			1000 // backoffMs
		)

		logger.info('Manifest download successful', { manifest })

		// Update manifest in store for preview
		manifestStore.setManifest(manifest)
		progress.value = 50
		statusMessage.value = 'Manifest downloaded. Ready to preview update.'
		// Load existing manifest for comparison if modpack path is selected
		const modpackPath = appStore.modpackPath
		if (modpackPath && modpackPath.trim().length > 0)
		{
			await executeWithRecovery(async () =>
			{
				// IMPROVED: Always generate manifest_old.json from minecraftinstance.json (actual installed state)
				progress.value = 60
				statusMessage.value = 'Generating manifest_old.json from current installation...'

				await generateManifestOldFromMinecraftInstance(modpackPath)
			}, 'generateManifestOld')

			// Write new manifest.json (target state)
			progress.value = 80
			statusMessage.value = 'Writing new manifest.json...'

			await executeWithRecovery(async () =>
			{
				await writeNewManifest(modpackPath, manifest)
			}, 'writeNewManifest')

			logger.info('Phase 1 complete: Manifest downloaded with improved backup system')
		}
		else
		{
			logger.info('Phase 1 complete: Manifest downloaded (no modpack path selected)')
		}

		statusMessage.value = 'Manifest ready for preview. Config files will be downloaded after confirmation.'
		statusType.value = 'success'
		currentOperation.value = null // Clear operation on success
	}
	catch (err)
	{
		handleError(err as Error, 'downloadFromGithub')
	}
	finally
	{
		downloading.value = false
		progress.value = 100
	}
}

// New function to download config files after user confirmation
async function downloadConfigFiles()
{
	if (uuid.value.trim().length === 0 || manifest.value === null)
	{
		return
	}

	statusMessage.value = 'Downloading config files...'
	statusType.value = 'info'
	progress.value = 0
	downloading.value = true

	try
	{
		const repo = appStore.githubRepo
		const { downloadConfigFiles } = useGithubApi()
		const configFiles = await downloadConfigFiles({
			repo,
			uuid: uuid.value.trim(),
			manifest: manifest.value,
			onProgress: (p, msg) =>
			{
				progress.value = p
				if (typeof msg === 'string' && msg.length > 0)
				{
					statusMessage.value = msg
				}
			}
		})

		downloadedConfigFiles.value = configFiles
		configFilesDownloaded.value = true

		// Write config files to disk if modpack path is selected
		const modpackPath = appStore.modpackPath
		if (modpackPath && modpackPath.trim().length > 0 && configFiles.length > 0)
		{
			const filesToWrite: Array<[string, string]> = []

			// Add config files
			for (const configFile of configFiles)
			{
				filesToWrite.push([configFile.relative_path, configFile.content])
			}

			// Write config files to modpack directory
			const writeSuccess = await writeFile(modpackPath, filesToWrite)
			if (!writeSuccess)
			{
				statusMessage.value = 'Config files downloaded but failed to write to disk.'
				statusType.value = 'warning'
				return
			}

			statusMessage.value = `Config files downloaded and written to ${modpackPath}`
		}
		else
		{
			statusMessage.value = configFiles.length > 0
				? 'Config files downloaded (no modpack path selected)'
				: 'No config files to download'
		}

		statusType.value = 'success'
	}
	catch (err)
	{
		const errorMessage = err instanceof Error ? err.message : 'Failed to download config files'
		statusMessage.value = errorMessage
		statusType.value = 'error'
		logger.error('Failed to download config files', { error: err, uuid: uuid.value, repo: appStore.githubRepo })
		throw new Error(`Config download failed: ${errorMessage}`) // Re-throw to be caught by confirmInstall
	}
	finally
	{
		downloading.value = false
		progress.value = 100
	}
}

async function installUpdate()
{
	if (!canInstall.value)
	{
		return
	}
	installing.value = true
	statusMessage.value = 'Starting installation...'
	statusType.value = 'info'
	progress.value = 0
	let unlisten: UnlistenFn | null = null
	try
	{
		unlisten = await listen('install-progress', (event) =>
		{
			const prog = (event as InstallProgressEvent).payload?.progress
			const message = (event as InstallProgressEvent).payload?.message
			if (typeof prog === 'number') progress.value = prog
			if (typeof message === 'string') statusMessage.value = message
		})
		const previousManifest = manifestStore.previousManifest
		if (manifest.value === null)
		{
			throw new Error('No manifest available for installation')
		}
		// Validate config files structure before installation
		const configFiles = downloadedConfigFiles.value
		for (const configFile of configFiles)
		{
			if (!configFile.filename || !configFile.relative_path || typeof configFile.content !== 'string')
			{
				throw new Error(`Invalid config file structure: ${JSON.stringify(configFile)}`)
			}
		}

		// Use optimized installation that only downloads/installs changed or new addons
		// This avoids redownloading unchanged addons, making updates much faster for large modpacks
		await installUpdateOptimized(
			appStore.modpackPath,
			previousManifest, // Can be null, optimized installer will handle it
			manifest.value,
			configFiles
		)
		statusMessage.value = previousManifest !== null ? 'Update installation complete!' : 'Fresh installation complete!'

		statusType.value = 'success'
	}
	catch (err)
	{
		statusMessage.value = (err instanceof Error ? err.message : 'Installation failed')
		logger.error(err)
		statusType.value = 'error'
	}
	finally
	{
		installing.value = false
		progress.value = 100
		if (typeof unlisten === 'function')
		{
			unlisten()
		}
	}
}

// Accessibility helper methods for button descriptions
const getDownloadButtonDescription = () =>
{
	if (downloading.value)
	{
		return 'download-disabled-help'
	}
	if (uuid.value.trim().length === 0)
	{
		return 'download-disabled-help'
	}
	return 'download-help'
}

const getInstallButtonDescription = () =>
{
	if (!canInstall.value)
	{
		return 'install-disabled-help'
	}
	return 'install-help'
}

// Progress bar helper functions
const getProgressLabel = () =>
{
	if (downloading.value)
	{
		return 'Downloading from GitHub...'
	}
	if (installing.value)
	{
		return 'Installing addons and config files...'
	}
	return ''
}

const getProgressColor = (): 'primary' | 'secondary' | 'accent' | 'success' | 'warning' | 'error' | 'info' =>
{
	if (downloading.value)
	{
		return 'info'
	}
	if (installing.value)
	{
		return 'primary'
	}
	if (progress.value === 100)
	{
		return 'success'
	}
	return 'primary'
}

// Navigation state management - fix for settings → dashboard bug
const route = useRoute()
const isNavigatingFromSettings = ref(false)

// Reset component state when navigating back from settings
const resetComponentState = () =>
{
	// Reset all local state that might be stale
	uuid.value = ''
	progress.value = 0
	statusMessage.value = ''
	statusType.value = 'info'
	downloading.value = false
	installing.value = false
	showPreview.value = false
	configFilesDownloaded.value = false
	downloadedConfigFiles.value = []
	currentOperation.value = null

	// Clear any errors
	clearError()

	logger.info('Component state reset after navigation')
}

// Watch for route changes to detect navigation back from settings
watch(() => route.name, (newRouteName, oldRouteName) =>
{
	if (oldRouteName === 'settings' && newRouteName === 'dashboard')
	{
		isNavigatingFromSettings.value = true
		resetComponentState()
		logger.info('Detected navigation from settings to dashboard, state reset')
	}
})

// Ensure component is properly initialized on mount
onMounted(() =>
{
	// If we just navigated from settings, ensure UI is in proper state
	if (isNavigatingFromSettings.value)
	{
		nextTick(() =>
		{
			isNavigatingFromSettings.value = false
		})
	}
	logger.info('UserPanel mounted')
})

// Cleanup on unmount
onUnmounted(() =>
{
	// Cancel any ongoing operations
	if (currentOperation.value !== null)
	{
		currentOperation.value = null
	}
	logger.info('UserPanel unmounted')
})

async function generateManifestOldFromMinecraftInstance(modpackPath: string)
{
	try
	{
		// Always generate manifest_old.json from minecraftinstance.json to represent actual installed state
		const minecraftInstancePath = `${modpackPath}/minecraftinstance.json`
		const minecraftInstanceContent = await readFile(minecraftInstancePath)

		if (minecraftInstanceContent !== null && minecraftInstanceContent.trim().length > 0)
		{
			logger.info('Generating manifest_old.json from minecraftinstance.json (actual installed state)')

			// Parse minecraftinstance.json into manifest format
			const parsedManifest = await parseMinecraftInstance(minecraftInstancePath)

			if (parsedManifest !== null)
			{
				// Write as manifest_old.json (represents current installed state)
				const oldManifestPath = `${modpackPath}/manifest_old.json`
				const manifestContent = JSON.stringify(parsedManifest, null, 2)
				const writeSuccess = await writeFile(oldManifestPath, manifestContent)

				if (!writeSuccess)
				{
					throw new Error('Failed to write manifest_old.json from minecraftinstance.json')
				}

				logger.info('Successfully generated manifest_old.json from actual installed state')

				// Load as previous manifest for comparison
				manifestStore.loadInstalledManifest(parsedManifest)

				return true
			}
			else
			{
				logger.error('Failed to parse minecraftinstance.json')
				throw new Error('Invalid minecraftinstance.json format')
			}
		}
		else
		{
			// No minecraftinstance.json found - fresh install
			logger.debug('No minecraftinstance.json found, treating as fresh install')
			manifestStore.loadInstalledManifest(null)
			return false
		}
	}
	catch (err)
	{
		logger.error('Failed to generate manifest_old.json from minecraftinstance.json', { err })
		// Fall back to fresh install
		manifestStore.loadInstalledManifest(null)
		return false
	}
}

async function writeNewManifest(modpackPath: string, newManifest: Manifest)
{
	try
	{
		// Simply write the new manifest.json (target state)
		const manifestPath = `${modpackPath}/manifest.json`
		const writeSuccess = await writeFile(manifestPath, JSON.stringify(newManifest, null, 2))

		if (!writeSuccess)
		{
			throw new Error('Failed to write new manifest.json')
		}

		logger.info('Successfully wrote new manifest.json')
		return true
	}
	catch (err)
	{
		logger.error('Failed to write new manifest.json', { err })
		throw err
	}
}
</script>
