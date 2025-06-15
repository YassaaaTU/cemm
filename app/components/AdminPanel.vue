<template>
  <div>
    <h2 class="text-2xl font-bold mb-4">
      Admin Mode
    </h2>
    <div class="flex gap-2 mt-4">
      <button
        class="btn btn-primary"
        @click="loadInstance"
      >
        Load Instance
      </button>
      <button
        class="btn btn-secondary"
        :disabled="manifest == null"
        @click="saveManifest"
      >
        Save Manifest
      </button>      <button
        class="btn btn-accent"
        :disabled="manifest == null || uploading"
        @click="uploadToGithub"
      >
        <span v-if="!uploading">
          Upload to GitHub
          <span
            v-if="selectedConfigFiles.length > 0"
            class="badge badge-secondary badge-sm ml-1"
          >
            +{{ selectedConfigFiles.length }} config
          </span>
        </span>        <span v-else>Uploading...</span>
      </button>
    </div>    <!-- Enhanced Error Handling -->
    <error-alert
      v-if="errorState.error"
      class="mt-4"
      :error-state="errorState"
      :retry-operation="retryCurrentOperation"
      @retry="retryCurrentOperation"
      @close="clearError"
    />

    <!-- Legacy Status Alert (for non-error messages) -->
    <status-alert
      v-else-if="statusMessage"
      class="mt-4"
      :message="statusMessage"
      :type="statusType"
    />

    <addon-list class="mt-4" />
    <manifest-preview class="mt-4" />

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
            <svg
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
              stroke-width="1.5"
              stroke="currentColor"
              class="w-4 h-4"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="M12 4.5v15m7.5-7.5h-15"
              />
            </svg>
            Add Config Files
          </button>
          <button
            class="btn btn-outline btn-sm"
            @click="selectConfigDirectory"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
              stroke-width="1.5"
              stroke="currentColor"
              class="w-4 h-4"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="M2.25 12.75V12A2.25 2.25 0 0 1 4.5 9.75h15A2.25 2.25 0 0 1 21.75 12v.75m-8.69-6.44-2.12-2.12a1.5 1.5 0 0 0-1.061-.44H4.5A2.25 2.25 0 0 0 2.25 6v12a2.25 2.25 0 0 0 2.25 2.25h15A2.25 2.25 0 0 0 21.75 18V9a2.25 2.25 0 0 0-2.25-2.25H11.69a1.5 1.5 0 0 0-1.061.44Z"
              />
            </svg>
            Add From Directory
          </button>
          <button
            v-if="selectedConfigFiles.length > 0"
            class="btn btn-outline btn-error btn-sm"
            @click="clearConfigFiles"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
              stroke-width="1.5"
              stroke="currentColor"
              class="w-4 h-4"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0"
              />
            </svg>
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
              <svg
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
                stroke-width="1.5"
                stroke="currentColor"
                class="w-4 h-4"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </button>
          </div>
        </div>

        <div
          v-else
          class="text-center py-6 opacity-60"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
            stroke-width="1.5"
            stroke="currentColor"
            class="w-12 h-12 mx-auto mb-2 opacity-40"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m2.25 0H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z"
            />
          </svg>
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
      <progress-bar :progress="progress" />
      <status-alert
        :message="statusMessage"
        :type="statusType"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import AddonList from '~/components/AddonList.vue'
import ErrorAlert from '~/components/ErrorAlert.vue'
import ManifestPreview from '~/components/ManifestPreview.vue'
import ProgressBar from '~/components/ProgressBar.vue'
import StatusAlert from '~/components/StatusAlert.vue'
import { useGithubApi } from '~/composables/useGithubApi'
import { useSecureStorage } from '~/composables/useSecureStorage'
import { useTauri } from '~/composables/useTauri'
import { useAppStore } from '~/stores/app'
import { useManifestStore } from '~/stores/manifest'
import type { ConfigFileWithContent, Manifest } from '~/types'
import { AppError, createErrorHandler, withNetworkRetry } from '~/utils/errorHandler'

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
			{
				// Extract filename and ask user for relative path
				const fileName = filePath.split(/[/\\]/).pop()
				if (fileName !== undefined && fileName.length > 0)
				{
					// For single file selection, prompt user for relative path or use config/ as default
					const defaultRelativePath = `config/${fileName}`
					// TODO: In a future enhancement, we could add a dialog to let user specify custom relative path

					newConfigFiles.push({
						filename: fileName,
						relative_path: defaultRelativePath,
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
	if (manifest.value == null)
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

			// Create updated manifest with config files included
			// Safe to assert non-null since we check at function start
			const currentManifest = manifest.value as Manifest
			const manifestWithConfig: Manifest = {
				mods: currentManifest.mods,
				resourcepacks: currentManifest.resourcepacks,
				shaderpacks: currentManifest.shaderpacks,
				datapacks: currentManifest.datapacks,
				config_files: selectedConfigFiles.value.map((cf) => ({
					filename: cf.filename,
					relative_path: cf.relative_path
				}))
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

			statusMessage.value = 'Upload successful!'
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
</script>
