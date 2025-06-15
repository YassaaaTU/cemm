<template>
  <div>
    <h2 class="text-2xl font-bold mb-4">
      User Mode
    </h2>
    <div class="form-control w-full max-w-xs">
      <label class="label">
        <span class="label-text">Update UUID</span>
      </label>
      <input
        v-model="uuid"
        type="text"
        class="input input-bordered w-full"
        placeholder="Paste update UUID here"
      />
    </div>
    <file-selector class="mt-4" />
    <div class="flex gap-2 mt-4">
      <button
        class="btn btn-primary"
        @click="openManifest"
      >
        Open Manifest
      </button>
      <button
        class="btn btn-secondary"
        :disabled="manifest == null"
        @click="saveManifest"
      >
        Save Manifest
      </button>
      <button
        class="btn btn-accent"
        :disabled="downloading || uuid.trim().length === 0"
        @click="downloadFromGithub"
      >
        <span v-if="!downloading">Download from GitHub</span>
        <span
          v-else
          class="loading loading-spinner"
        />
      </button>
      <button
        class="btn btn-success"
        :disabled="!canInstall"
        @click="showPreview = true"
      >
        <span v-if="!installing">Install Update</span>
        <span
          v-else
          class="loading loading-spinner"
        />
      </button>
    </div>
    <manifest-preview class="mt-4" />
    <div
      v-if="manifest && manifest.datapacks && manifest.datapacks.length > 0"
      class="mt-4"
    >
      <strong>Datapacks:</strong>
      <ul class="list-disc ml-6">
        <li
          v-for="datapack in manifest.datapacks"
          :key="datapack.addon_project_id"
        >
          {{ datapack.addon_name }} ({{ datapack.version }})
        </li>
      </ul>
    </div>
    <addon-list class="mt-4" />
    <div class="mt-6 flex flex-col gap-2">
      <progress-bar :progress="progress" />
      <status-alert
        :message="statusMessage"
        :type="statusType"
      />
    </div>

    <!-- Update Preview Modal -->
    <update-preview
      v-if="showPreview && previewData"
      :preview="previewData"
      :installing="installing"
      @close="showPreview = false"
      @confirm="confirmInstall"
    />
  </div>
</template>

<script setup lang="ts">
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import AddonList from '~/components/AddonList.vue'
import FileSelector from '~/components/FileSelector.vue'
import ManifestPreview from '~/components/ManifestPreview.vue'
import ProgressBar from '~/components/ProgressBar.vue'
import StatusAlert from '~/components/StatusAlert.vue'
import UpdatePreview from '~/components/UpdatePreview.vue'
import type { ConfigFile } from '~/composables/useGithubApi'
import { useGithubApi } from '~/composables/useGithubApi'
import { useTauri } from '~/composables/useTauri'
import { useAppStore } from '~/stores/app'
import { useManifestStore } from '~/stores/manifest'
import type { Manifest } from '~/types'

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
const { selectFile, writeFile, readFile, parseMinecraftInstance } = useTauri()
const manifestStore = useManifestStore()
const manifest = computed(() => manifestStore.manifest)
const { downloadUpdate } = useGithubApi()
const appStore = useAppStore()
const downloading = ref(false)
const installing = ref(false)
const downloadedConfigFiles = ref<ConfigFile[]>([])
const logger = usePinoLogger()
const { installUpdate: tauriInstallUpdate, installUpdateWithCleanup } = useTauri()

const canInstall = computed(() =>
	manifest.value !== null
	&& appStore.modpackPath.trim().length > 0
	&& !installing.value
	&& !downloading.value
)

const showPreview = ref(false)

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
			oldAddons: Array<{ addon_project_id: number, addon_name: string, version: string }>,
			newAddons: Array<{ addon_project_id: number, addon_name: string, version: string }>
		) =>
		{
			// Find removed addons
			for (const oldAddon of oldAddons)
			{
				const found = newAddons.find((a) => a.addon_project_id === oldAddon.addon_project_id)
				if (found === undefined)
				{
					diff.removed_addons.push(oldAddon.addon_name)
				}
			}

			// Find updated addons
			for (const oldAddon of oldAddons)
			{
				const newAddon = newAddons.find((a) => a.addon_project_id === oldAddon.addon_project_id)
				if (newAddon !== undefined && oldAddon.version !== newAddon.version)
				{
					diff.updated_addons.push([oldAddon.version, newAddon.version])
				}
			}

			// Find new addons
			for (const newAddon of newAddons)
			{
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
		// Fresh install - everything is new
		diff.new_addons = [
			...newManifest.mods.map((addon) => addon.addon_name),
			...newManifest.resourcepacks.map((addon) => addon.addon_name),
			...newManifest.shaderpacks.map((addon) => addon.addon_name),
			...newManifest.datapacks.map((addon) => addon.addon_name)
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
		hasChanges: oldManifest === null ? false : hasChanges
	}
})

function confirmInstall()
{
	showPreview.value = false
	installUpdate()
}

async function openManifest()
{
	statusMessage.value = ''
	const filePath = await selectFile()
	if (filePath == null || filePath.length === 0)
	{
		statusMessage.value = 'No file selected.'
		statusType.value = 'warning'
		return
	}
	const content = await readFile(filePath)
	if (content == null || content.length === 0)
	{
		statusMessage.value = 'Failed to read file.'
		statusType.value = 'error'
		return
	}
	try
	{
		const parsed = JSON.parse(content)
		manifestStore.setManifest(parsed)
		statusMessage.value = 'Manifest loaded.'
		statusType.value = 'success'
	}
	catch (_err)
	{
		statusMessage.value = 'Invalid manifest format.'
		statusType.value = 'error'
	}
}

async function saveManifest()
{
	statusMessage.value = ''
	if (manifest.value == null)
	{
		return
	}
	const filePath = await selectFile()
	if (filePath == null || filePath.length === 0)
	{
		statusMessage.value = 'No file selected.'
		statusType.value = 'warning'
		return
	}
	const ok = await writeFile(filePath, JSON.stringify(manifest.value, null, 2))
	if (ok)
	{
		statusMessage.value = 'Manifest saved.'
		statusType.value = 'success'
	}
	else
	{
		statusMessage.value = 'Failed to save file.'
		statusType.value = 'error'
	}
}

async function backupAndReplaceManifest(modpackPath: string, newManifest: Manifest)
{
	const manifestPath = `${modpackPath}/manifest.json`
	const oldManifestPath = `${modpackPath}/manifest_old.json`

	try
	{
		// Check if current manifest.json exists
		const currentManifestContent = await readFile(manifestPath)

		if (currentManifestContent !== null && currentManifestContent.trim().length > 0)
		{
			// Delete any existing manifest_old.json
			// Note: writeFile with single file will overwrite, so we don't need to manually delete

			// Move current manifest.json to manifest_old.json
			const backupSuccess = await writeFile(oldManifestPath, currentManifestContent)
			if (!backupSuccess)
			{
				throw new Error('Failed to backup current manifest')
			}
			logger.info('Backed up current manifest to manifest_old.json')
		}

		// Write new manifest as manifest.json
		const writeSuccess = await writeFile(manifestPath, JSON.stringify(newManifest, null, 2))
		if (!writeSuccess)
		{
			throw new Error('Failed to write new manifest')
		}

		logger.info('Successfully updated manifest.json')
		return true
	}
	catch (err)
	{
		logger.error('Failed to backup and replace manifest', { err })
		throw err
	}
}

/**
 * Try to load manifest from minecraftinstance.json when no manifest.json exists
 * This allows updating from raw Minecraft instances
 */
async function tryLoadFromMinecraftInstance(modpackPath: string)
{
	try
	{
		// Check for minecraftinstance.json
		const minecraftInstancePath = `${modpackPath}/minecraftinstance.json`
		const minecraftInstanceContent = await readFile(minecraftInstancePath)

		if (minecraftInstanceContent !== null && minecraftInstanceContent.trim().length > 0)
		{
			logger.info('Found minecraftinstance.json, parsing to manifest format')
			statusMessage.value = 'Converting minecraftinstance.json to manifest format...' // Parse minecraftinstance.json into manifest format
			const parsedManifest = await parseMinecraftInstance(minecraftInstancePath)

			if (parsedManifest !== null)
			{
				logger.info('Successfully parsed minecraftinstance.json to manifest')

				// Step 1: Write the parsed manifest as manifest.json
				const manifestPath = `${modpackPath}/manifest.json`
				const manifestContent = JSON.stringify(parsedManifest, null, 2)
				const writeSuccess = await writeFile(manifestPath, manifestContent)

				if (!writeSuccess)
				{
					throw new Error('Failed to write parsed manifest.json')
				}

				logger.info('Created manifest.json from minecraftinstance.json')

				// Step 2: Immediately rename it to manifest_old.json for backup
				const oldManifestPath = `${modpackPath}/manifest_old.json`
				const backupSuccess = await writeFile(oldManifestPath, manifestContent)

				if (!backupSuccess)
				{
					throw new Error('Failed to backup parsed manifest as manifest_old.json')
				}

				logger.info('Backed up parsed manifest as manifest_old.json')

				// Step 3: Load the parsed manifest as the previous manifest for comparison
				manifestStore.loadInstalledManifest(parsedManifest)

				statusMessage.value = 'Converted Minecraft instance to manifest format for update comparison'
				return
			}
			else
			{
				logger.error('Failed to parse minecraftinstance.json')
				throw new Error('Invalid minecraftinstance.json format')
			}
		}
		else
		{
			// No minecraftinstance.json found either
			logger.debug('No minecraftinstance.json found, proceeding with fresh install')
			manifestStore.loadInstalledManifest(null)
		}
	}
	catch (err)
	{
		logger.error('Failed to process minecraftinstance.json', { err })
		// Fall back to fresh install
		manifestStore.loadInstalledManifest(null)
	}
}

async function downloadFromGithub()
{
	if (uuid.value.trim().length === 0)
	{
		statusMessage.value = 'Please enter a valid UUID.'
		statusType.value = 'warning'
		return
	}
	statusMessage.value = ''
	statusType.value = 'info'
	progress.value = 0
	downloading.value = true
	try
	{
		const repo = appStore.githubRepo
		if (repo.trim().length === 0)
		{
			statusMessage.value = 'GitHub repo not set.'
			statusType.value = 'error'
			downloading.value = false
			return
		}
		const result = await downloadUpdate({
			repo,
			uuid: uuid.value.trim(),
			onProgress: (p, msg) =>
			{
				progress.value = p
				if (typeof msg === 'string' && msg.length > 0)
				{
					statusMessage.value = msg
				}
			}
		})

		// Update manifest in store for preview
		manifestStore.setManifest(result.manifest)
		downloadedConfigFiles.value = result.configFiles

		// Write files to disk if modpack path is selected
		const modpackPath = appStore.modpackPath
		if (modpackPath && modpackPath.trim().length > 0)
		{
			progress.value = 80
			statusMessage.value = 'Writing files to disk...' // Enhanced manifest loading logic for update workflow
			try
			{
				// Step 1: Try to read existing manifest.json from the modpack directory
				const manifestPath = `${modpackPath}/manifest.json`
				const existingManifestContent = await readFile(manifestPath)

				if (existingManifestContent !== null && existingManifestContent.trim().length > 0)
				{
					// Found existing manifest.json - use it as previous manifest
					const existingManifest = JSON.parse(existingManifestContent)
					manifestStore.loadInstalledManifest(existingManifest)
					logger.info('Loaded existing manifest.json for update comparison', { manifestPath })
				}
				else
				{
					// Step 2: No manifest.json found, check for minecraftinstance.json
					await tryLoadFromMinecraftInstance(modpackPath)
				}
			}
			catch (err)
			{
				// Step 3: manifest.json reading failed, try minecraftinstance.json fallback
				logger.debug('Failed to read manifest.json, trying minecraftinstance.json fallback', { err })
				await tryLoadFromMinecraftInstance(modpackPath)
			}

			// Backup current manifest and write new one using our new system
			await backupAndReplaceManifest(modpackPath, result.manifest)

			// Prepare other files for batch write (config files only now)
			const filesToWrite: Array<[string, string]> = []

			// Add config files (manifest.json is handled by backupAndReplaceManifest)
			for (const configFile of result.configFiles)
			{
				filesToWrite.push([configFile.path, configFile.content])
			}

			// Write config files to modpack directory (if any)
			if (filesToWrite.length > 0)
			{
				const writeSuccess = await writeFile(modpackPath, filesToWrite)
				if (!writeSuccess)
				{
					statusMessage.value = 'Downloaded manifest successfully, but failed to write config files to disk.'
					statusType.value = 'warning'
					return
				}
			}

			const totalFiles = filesToWrite.length + 1 // +1 for manifest.json
			statusMessage.value = `Download successful! ${totalFiles} files written to ${modpackPath}. Previous manifest saved as manifest_old.json`
			statusType.value = 'success'
		}
		else
		{
			statusMessage.value = 'Download successful! (No modpack path selected - files not written to disk)'
			statusType.value = 'warning'
		}
	}
	catch (err)
	{
		statusMessage.value = (err instanceof Error ? err.message : 'Download failed')
		statusType.value = 'error'
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

		if (previousManifest !== null)
		{
			// Use update with cleanup for existing installations
			await installUpdateWithCleanup(
				appStore.modpackPath,
				previousManifest,
				manifest.value,
				downloadedConfigFiles.value
			)
			statusMessage.value = 'Update installation complete!'
		}
		else
		{
			// Use regular install for fresh installations
			await tauriInstallUpdate(
				appStore.modpackPath,
				manifest.value,
				downloadedConfigFiles.value
			)
			statusMessage.value = 'Fresh installation complete!'
		}

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
</script>
