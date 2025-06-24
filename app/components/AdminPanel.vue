<template>
  <div
    role="main"
    aria-labelledby="admin-mode-title"
  >
    <h2
      id="admin-mode-title"
      class="text-2xl font-bold mb-4"
    >
      Admin Mode
    </h2>

    <!-- Action Buttons Section -->
    <section
      aria-labelledby="admin-actions-section"
      class="mb-6"
    >
      <div class="flex flex-wrap gap-2">
        <button
          class="btn btn-primary"
          aria-describedby="load-instance-help"
          @click="loadInstance"
        >
          Load Instance
        </button>

        <button
          class="btn btn-secondary"
          :disabled="manifest == null"
          :aria-describedby="manifest == null ? 'save-disabled-help' : 'save-manifest-help'"
          @click="saveManifest"
        >
          Save Manifest
        </button>        <button
          class="btn btn-accent"
          :disabled="(manifest == null && selectedConfigFiles.length === 0) || uploading"
          :aria-describedby="getUploadButtonDescription()"
          @click="uploadToGithub"
        >
          <span v-if="!uploading">
            {{ getUploadButtonText() }}
            <span
              v-if="selectedConfigFiles.length > 0"
              class="badge badge-secondary badge-sm ml-1"
              :aria-label="`${selectedConfigFiles.length} config files selected`"
            >
              +{{ selectedConfigFiles.length }} config
            </span>
          </span>
          <loading-spinner
            v-else
            :loading="true"
            size="sm"
            message="Uploading..."
            aria-label="Uploading manifest and config files to GitHub"
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
        :update-info="manifestStore.updateInfo"
        title="Mods"
        category="mods"
        class="mb-4"
      />
      <addon-list
        v-if="manifest.resourcepacks.length > 0"
        :addons="manifest.resourcepacks"
        :update-info="manifestStore.updateInfo"
        title="Resource Packs"
        category="resourcepacks"
        class="mb-4"
      />
      <addon-list
        v-if="manifest.shaderpacks.length > 0"
        :addons="manifest.shaderpacks"
        :update-info="manifestStore.updateInfo"
        title="Shader Packs"
        category="shaderpacks"
        class="mb-4"
      />
      <addon-list
        v-if="manifest.datapacks.length > 0"
        :addons="manifest.datapacks"
        :update-info="manifestStore.updateInfo"
        title="Data Packs"
        category="datapacks"
      />
    </div>

    <div
      v-else
      class="space-y-4"
    >
      <!-- Skeleton loading state -->
      <div class="flex items-center justify-center p-8 text-center">
        <div class="space-y-3">
          <div class="text-gray-400 text-sm">
            No manifest loaded
          </div>
          <div class="text-xs opacity-60">
            Load a minecraftinstance.json file to see addon details
          </div>
          <!-- Skeleton placeholder -->
          <div class="space-y-2 mt-4">
            <div class="skeleton h-4 w-32 mx-auto" />
            <div class="skeleton h-3 w-24 mx-auto" />
            <div class="skeleton h-3 w-28 mx-auto" />
          </div>
        </div>
      </div>
    </div>

    <!-- Config Files Section -->
    <div class="mt-6 card bg-base-200 shadow-lg">
      <div class="card-body">
        <h3 class="card-title text-lg">
          Config Files (Optional)
        </h3>
        <p class="text-sm opacity-70 mb-4">
          Select configuration files to include with your modpack update.
        </p>        <div class="flex gap-2 mb-4">
          <button
            class="btn btn-outline btn-sm"
            @click="selectConfigFiles"
          >
            <Icon
              name="mdi:file-plus"
              size="1.2rem"
              class="mr-1"
            />
            Add Config Files
          </button>
          <button
            class="btn btn-outline btn-sm"
            @click="selectConfigDirectory"
          >
            <Icon
              name="mdi:folder-plus"
              size="1.2rem"
              class="mr-1"
            />
            Add From Directory
          </button>
          <button
            v-if="selectedConfigFiles.length > 0"
            class="btn btn-outline btn-error btn-sm"
            @click="clearConfigFiles"
          >
            <Icon
              name="mdi:trash-can"
              size="1.2rem"
              class="mr-1"
            />
            Clear All
          </button>
        </div>

        <!-- Selected Config Files List -->
        <div
          v-if="selectedConfigFiles.length > 0"
          class="space-y-2"
        >
          <div class="text-sm font-medium opacity-80">
            Selected Files ({{ selectedConfigFiles.length }}):
          </div>
          <div
            v-for="(configFile, index) in selectedConfigFiles"
            :key="index"
            class="flex items-center justify-between p-3 bg-base-100 rounded-lg"
          >
            <div class="flex items-center gap-3">
              <div class="badge badge-primary badge-sm">
                CONFIG
              </div>
              <span class="font-mono text-sm">{{ configFile.relative_path }}</span>
              <span class="text-xs opacity-60">
                ({{ Math.round(configFile.content.length / 1024 * 100) / 100 }} KB)
              </span>
            </div>
            <button
              class="btn btn-ghost btn-xs btn-circle"
              @click="removeConfigFile(configFile)"
            >
              <Icon
                name="mdi:close"
                size="1.2rem"
                class="text-error"
              />
            </button>
          </div>
        </div>

        <div
          v-else
          class="text-center py-6 opacity-60"
        >
          <Icon
            name="mdi:file-document-outline"
            size="4rem"
            class="text-gray-400 mb-2"
          />
          <p class="text-sm">
            No config files selected
          </p>
          <p class="text-xs opacity-60">
            Config files will be applied to the user's modpack directory
          </p>
        </div>
      </div>
    </div>
    <div class="mt-6 flex flex-col gap-2">
      <progress-bar
        :progress="progress"
        :label="uploading ? 'Uploading to GitHub...' : ''"
        :color="uploading ? 'primary' : 'success'"
        :show-percentage="uploading"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import type { ConfigFileWithContent, Manifest } from '~/types'

const { uploadUpdate } = useGithubApi()
const { getSecure } = useSecureStorage()
const appStore = useAppStore()
const uploading = ref(false)

const progress = ref(0)
const statusMessage = ref('')
const statusType = ref<'success' | 'error' | 'info' | 'warning'>('info')

// Enhanced error handling
const logger = useNuxtApp().$logger
const { errorState, handleError, retry, clearError } = createErrorHandler(
	statusMessage,
	statusType,
	logger
)

// Track current operation for retry functionality
const currentOperation = ref<(() => Promise<void>) | null>(null)
const retryCurrentOperation = async () =>
{
	if (currentOperation.value !== null)
	{
		await retry(currentOperation.value)
	}
}

// Config file management
const selectedConfigFiles = ref<ConfigFileWithContent[]>([])

const { selectFile, selectSaveFile, selectMultipleFiles, selectConfigDirectory: selectDirectory, readDirectoryRecursive, writeFile, parseMinecraftInstance, compareManifests, readFile } = useTauri()
const manifestStore = useManifestStore()
const manifest = computed(() => manifestStore.manifest)

// UI helper functions for upload button
const getUploadButtonText = () =>
{
	if (manifest.value !== null && selectedConfigFiles.value.length > 0)
	{
		return 'Upload to GitHub'
	}
	else if (manifest.value !== null)
	{
		return 'Upload to GitHub'
	}
	else if (selectedConfigFiles.value.length > 0)
	{
		return 'Upload Config Only'
	}
	return 'Upload to GitHub'
}

const getUploadButtonDescription = () =>
{
	if (uploading.value)
	{
		return 'upload-disabled-help'
	}
	if (manifest.value == null && selectedConfigFiles.value.length === 0)
	{
		return 'upload-disabled-help'
	}
	return 'upload-help'
}

// Config file selection functions
async function selectConfigFiles()
{
	statusMessage.value = ''
	const filePaths = await selectMultipleFiles()

	if (filePaths.length === 0)
	{
		statusMessage.value = 'No config files selected.'
		statusType.value = 'warning'
		return
	}
	try
	{
		const newConfigFiles: ConfigFileWithContent[] = []

		for (const filePath of filePaths)
		{
			const content = await readFile(filePath)
			if (content !== null)
			{ // Extract filename and calculate relative path from modpack root
				const fileName = filePath.split(/[/\\]/).pop()
				if (fileName !== undefined && fileName.length > 0)
				{
				// Calculate relative path from modpack directory
					let relativePath = fileName // Default to root if we can't determine
					const modpackPath = appStore.modpackPath

					if (modpackPath && filePath.startsWith(modpackPath))
					{
					// Calculate actual relative path from modpack root
						const normalizedModpackPath = modpackPath.replace(/\\/g, '/')
						const normalizedFilePath = filePath.replace(/\\/g, '/')
						relativePath = normalizedFilePath.substring(normalizedModpackPath.length + 1)
					}

					newConfigFiles.push({
						filename: fileName,
						relative_path: relativePath,
						content
					})
				}
			}
		}

		selectedConfigFiles.value = [...selectedConfigFiles.value, ...newConfigFiles]
		statusMessage.value = `Added ${newConfigFiles.length} config file(s).`
		statusType.value = 'success'
	}
	catch (err)
	{
		statusMessage.value = `Failed to read config files: ${err instanceof Error ? err.message : 'Unknown error'}`
		statusType.value = 'error'
	}
}

function removeConfigFile(configFile: ConfigFileWithContent)
{
	const index = selectedConfigFiles.value.findIndex((cf) => cf.relative_path === configFile.relative_path)
	if (index !== -1)
	{
		selectedConfigFiles.value.splice(index, 1)
		statusMessage.value = `Removed config file: ${configFile.relative_path}`
		statusType.value = 'info'
	}
}

function clearConfigFiles()
{
	selectedConfigFiles.value = []
	statusMessage.value = 'Cleared all config files.'
	statusType.value = 'info'
}

// Load a minecraftinstance.json and convert to manifest
async function loadInstance()
{
	const loadOperation = async () =>
	{
		statusMessage.value = ''

		// Select minecraftinstance.json
		const filePath = await selectFile()
		if (filePath == null || filePath.length === 0)
		{
			statusMessage.value = 'No file selected.'
			statusType.value = 'warning'
			return
		}

		try
		{
			// Parse minecraftinstance.json to Manifest
			const parsed = await parseMinecraftInstance(filePath)
			if (parsed == null)
			{
				throw new AppError('INVALID_MANIFEST', 'Failed to parse minecraftinstance.json')
			}

			// Save previous manifest for diffing (store in Pinia)
			if (manifest.value != null)
			{
				manifestStore.setPreviousManifest(manifest.value)
			}
			manifestStore.setManifest(parsed)
			statusMessage.value = 'Manifest generated from minecraftinstance.json.'
			statusType.value = 'success'

			// If previous manifest exists, show diff (store in Pinia)
			if (manifestStore.previousManifest != null)
			{
				const diff = await compareManifests(manifestStore.previousManifest, parsed)
				manifestStore.setUpdateInfo(diff)
			}
			else
			{
				manifestStore.setUpdateInfo(null)
			}
		}
		catch (error)
		{
			if (error instanceof AppError)
			{
				handleError(error)
			}
			else
			{
				handleError(new AppError('FILE_READ_ERROR', error instanceof Error ? error : new Error(String(error))))
			}
		}
	}

	// Set current operation for retry functionality
	currentOperation.value = loadOperation
	await loadOperation()
}

// Save/export the generated manifest
async function saveManifest()
{
	statusMessage.value = ''
	if (manifest.value == null)
	{
		return
	}
	// Prompt user for save location (Save As dialog)
	const filePath = await selectSaveFile()
	if (filePath == null || filePath.length === 0)
	{
		statusMessage.value = 'No file selected.'
		statusType.value = 'warning'
		return
	}
	// Check if file exists (optional, for user feedback)
	let fileExists = false
	try
	{
		const existing = await readFile(filePath)
		if (typeof existing === 'string' && existing.length > 0)
		{
			fileExists = true
		}
	}
	catch (_err)
	{
		// File does not exist, proceed
	}
	if (fileExists)
	{
		statusMessage.value = 'File already exists. Overwriting.'
		statusType.value = 'warning'
	}
	const ok = await writeFile(filePath, JSON.stringify(manifest.value, null, 2))
	if (ok)
	{
		statusMessage.value = `Manifest saved as ${filePath}.`
		statusType.value = 'success'
	}
	else
	{
		statusMessage.value = 'Failed to save manifest.'
		statusType.value = 'error'
	}
}

async function uploadToGithub()
{
	// Allow upload if we have either a manifest OR config files
	if (manifest.value == null && selectedConfigFiles.value.length === 0)
	{
		return
	}

	const uploadOperation = async () =>
	{
		statusMessage.value = ''
		statusType.value = 'info'
		progress.value = 0
		uploading.value = true
		try
		{
			// Get repo and token
			const repo = appStore.githubRepo
			const token = await getSecure('cemm_github_token')
			if (repo.trim().length === 0 || token == null || token.trim().length === 0)
			{
				throw new Error('MISSING_GITHUB_SETTINGS')
			}
			// Generate UUID (for now, use Date.now as stub)
			const uuid = Date.now().toString()

			// Create manifest (either from existing or config-only)
			let manifestWithConfig: Manifest
			if (manifest.value !== null)
			{
				// Full manifest with addons + config files
				const currentManifest = manifest.value as Manifest
				manifestWithConfig = {
					updateType: 'full',
					mods: currentManifest.mods,
					resourcepacks: currentManifest.resourcepacks,
					shaderpacks: currentManifest.shaderpacks,
					datapacks: currentManifest.datapacks,
					config_files: selectedConfigFiles.value.map((cf) => ({
						filename: cf.filename,
						relative_path: cf.relative_path
					}))
				}
			}
			else
			{
				// Config-only manifest (empty addons)
				manifestWithConfig = {
					updateType: 'config',
					mods: [],
					resourcepacks: [],
					shaderpacks: [],
					datapacks: [],
					config_files: selectedConfigFiles.value.map((cf) => ({
						filename: cf.filename,
						relative_path: cf.relative_path
					}))
				}
			}

			// Use selected config files and wrap with network retry
			const configFiles = selectedConfigFiles.value
			await withNetworkRetry(async () =>
			{
				await uploadUpdate({
					repo,
					token,
					uuid,
					manifest: manifestWithConfig,
					configFiles,
					onProgress: (p, msg) =>
					{
						progress.value = p
						if (typeof msg === 'string' && msg.length > 0)
						{
							statusMessage.value = msg
						}
					}
				})
			})

			statusMessage.value = manifest.value !== null
				? 'Upload successful!'
				: 'Config files uploaded successfully!'
			statusType.value = 'success'
		}
		catch (error)
		{
			const errorCode = error instanceof Error && error.message.startsWith('GITHUB_')
				? error.message
				: 'GITHUB_UPLOAD_FAILED'
			const errorToHandle = error instanceof Error ? error : new Error(String(error))
			handleError(new AppError(errorCode, errorToHandle), 'GitHub upload')
		}
		finally
		{
			uploading.value = false
			progress.value = 100
		}
	}
	// Set current operation for retry functionality
	currentOperation.value = uploadOperation
	await uploadOperation()
}

// New function for directory-based config file selection
async function selectConfigDirectory()
{
	statusMessage.value = ''
	const dirPath = await selectDirectory()

	if (dirPath === null)
	{
		statusMessage.value = 'No directory selected.'
		statusType.value = 'warning'
		return
	}
	try
	{
		statusMessage.value = 'Scanning directory for config files...'
		statusType.value = 'info'

		// Calculate the parent directory to use as base path
		// This ensures the selected directory name is included in relative paths
		// Handle both Windows (\) and Unix (/) path separators
		const lastBackslash = dirPath.lastIndexOf('\\')
		const lastForwardslash = dirPath.lastIndexOf('/')
		const lastSeparator = Math.max(lastBackslash, lastForwardslash)

		let parentPath: string
		if (lastSeparator > 0)
		{
			parentPath = dirPath.substring(0, lastSeparator)
		}
		else
		{
			// If no separator found, use the directory itself as fallback
			parentPath = dirPath
		}

		// Use the selected directory for scanning, but parent directory as base for relative paths
		const configFiles = await readDirectoryRecursive(dirPath, parentPath)

		if (configFiles.length === 0)
		{
			statusMessage.value = 'No config files found in the selected directory.'
			statusType.value = 'warning'
			return
		}

		// Add the found config files to the selection
		selectedConfigFiles.value = [...selectedConfigFiles.value, ...configFiles]
		statusMessage.value = `Added ${configFiles.length} config file(s) from directory.`
		statusType.value = 'success'
	}
	catch (err)
	{
		statusMessage.value = `Failed to read config files from directory: ${err instanceof Error ? err.message : 'Unknown error'}`
		statusType.value = 'error'
	}
}

// Navigation state management - fix for settings → dashboard bug
const route = useRoute()

// Reset component state when navigating back from settings
const resetComponentState = () =>
{
	// Reset all local state that might be stale
	uploading.value = false
	progress.value = 0
	statusMessage.value = ''
	statusType.value = 'info'
	currentOperation.value = null

	// Clear any errors
	clearError()

	console.info('AdminPanel state reset after navigation')
}

// Watch for route changes to detect navigation back from settings
watch(() => route.name, (newRouteName, oldRouteName) =>
{
	if (oldRouteName === 'settings' && newRouteName === 'dashboard')
	{
		resetComponentState()
		console.info('Detected navigation from settings to dashboard, AdminPanel state reset')
	}
})

// Ensure component is properly initialized on mount
onMounted(() =>
{
	console.info('AdminPanel mounted')
})

// Cleanup on unmount
onUnmounted(() =>
{
	// Cancel any ongoing operations
	if (currentOperation.value !== null)
	{
		currentOperation.value = null
	}
	console.info('AdminPanel unmounted')
})
</script>
