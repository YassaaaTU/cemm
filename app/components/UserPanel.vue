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
    </div>    <!-- Install Preview Modal -->
    <dialog
      v-if="showPreview"
      open
      class="modal modal-open"
    >
      <div class="modal-box max-w-3xl">
        <h3 class="font-bold text-lg mb-2">
          {{ isUpdate ? 'Update Preview' : 'Install Preview' }}
        </h3>
        <p class="mb-4">
          {{ isUpdate ? 'The following changes will be applied to' : 'The following will be installed to' }}
          <span class="font-mono">{{ appStore.modpackPath }}</span>:
        </p>

        <!-- Update-specific diff view -->
        <div
          v-if="isUpdate && updateDiff"
          class="mb-4"
        >
          <!-- Removed addons -->
          <div
            v-if="updateDiff.removedAddons.length > 0"
            class="mb-3"
          >
            <h4 class="font-semibold text-error mb-1">
              🗑️ Will be removed:
            </h4>
            <ul class="list-disc ml-6 text-error">
              <li
                v-for="addon in updateDiff.removedAddons"
                :key="addon"
              >
                {{ addon }}
              </li>
            </ul>
          </div>

          <!-- Updated addons -->
          <div
            v-if="updateDiff.updatedAddons.length > 0"
            class="mb-3"
          >
            <h4 class="font-semibold text-warning mb-1">
              🔄 Will be updated:
            </h4>
            <ul class="list-disc ml-6 text-warning">
              <li
                v-for="addon in updateDiff.updatedAddons"
                :key="addon.name"
              >
                {{ addon.name }}: {{ addon.oldVersion }} → {{ addon.newVersion }}
              </li>
            </ul>
          </div>

          <!-- New addons -->
          <div
            v-if="updateDiff.newAddons.length > 0"
            class="mb-3"
          >
            <h4 class="font-semibold text-success mb-1">
              ➕ Will be added:
            </h4>
            <ul class="list-disc ml-6 text-success">
              <li
                v-for="addon in updateDiff.newAddons"
                :key="addon"
              >
                {{ addon }}
              </li>
            </ul>
          </div>

          <!-- Summary -->
          <div class="bg-base-200 p-3 rounded mb-3">
            <strong>Summary:</strong>
            {{ updateDiff.removedAddons.length }} removed,
            {{ updateDiff.updatedAddons.length }} updated,
            {{ updateDiff.newAddons.length }} new
          </div>
        </div>

        <!-- Fresh install view -->
        <div
          v-else
          class="mb-4"
        >
          <div class="mb-2">
            <strong>Mods ({{ manifest?.mods?.length || 0 }}):</strong>
            <ul class="list-disc ml-6 max-h-32 overflow-y-auto">
              <li
                v-for="mod in manifest?.mods ?? []"
                :key="mod.addon_project_id"
              >
                {{ mod.addon_name }} ({{ mod.version }})
              </li>
            </ul>
          </div>
          <div class="mb-2">
            <strong>Resourcepacks ({{ manifest?.resourcepacks?.length || 0 }}):</strong>
            <ul class="list-disc ml-6 max-h-32 overflow-y-auto">
              <li
                v-for="rp in manifest?.resourcepacks ?? []"
                :key="rp.addon_project_id"
              >
                {{ rp.addon_name }} ({{ rp.version }})
              </li>
            </ul>
          </div>
          <div class="mb-2">
            <strong>Shaderpacks ({{ manifest?.shaderpacks?.length || 0 }}):</strong>
            <ul class="list-disc ml-6 max-h-32 overflow-y-auto">
              <li
                v-for="sp in manifest?.shaderpacks ?? []"
                :key="sp.addon_project_id"
              >
                {{ sp.addon_name }} ({{ sp.version }})
              </li>
            </ul>
          </div>
          <div class="mb-2">
            <strong>Datapacks ({{ manifest?.datapacks?.length || 0 }}):</strong>
            <ul class="list-disc ml-6 max-h-32 overflow-y-auto">
              <li
                v-for="dp in manifest?.datapacks ?? []"
                :key="dp.addon_project_id"
              >
                {{ dp.addon_name }} ({{ dp.version }})
              </li>
            </ul>
          </div>
        </div>

        <!-- Config files (always shown) -->
        <div
          v-if="downloadedConfigFiles.length > 0"
          class="mb-4"
        >
          <strong>Config files ({{ downloadedConfigFiles.length }}):</strong>
          <ul class="list-disc ml-6 max-h-24 overflow-y-auto">
            <li
              v-for="cf in downloadedConfigFiles"
              :key="cf.path"
            >
              {{ cf.path }}
            </li>
          </ul>
        </div>

        <div class="modal-action mt-4 flex gap-2">
          <button
            class="btn btn-success"
            :disabled="installing"
            @click="confirmInstall"
          >
            {{ isUpdate ? 'Confirm Update' : 'Confirm Install' }}
          </button>
          <button
            class="btn btn-ghost"
            :disabled="installing"
            @click="showPreview = false"
          >
            Cancel
          </button>
        </div>
      </div>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import AddonList from '~/components/AddonList.vue'
import FileSelector from '~/components/FileSelector.vue'
import ManifestPreview from '~/components/ManifestPreview.vue'
import ProgressBar from '~/components/ProgressBar.vue'
import StatusAlert from '~/components/StatusAlert.vue'
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
const { selectFile, writeFile, readFile } = useTauri()
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

// Calculate update diff for preview
const updateDiff = computed(() =>
{
	if (manifest.value === null || manifestStore.previousManifest === null)
	{
		return null
	}

	const old = manifestStore.previousManifest
	const newManifest = manifest.value

	const diff = {
		removedAddons: [] as string[],
		updatedAddons: [] as Array<{ name: string, oldVersion: string, newVersion: string }>,
		newAddons: [] as string[]	}

	// Type alias for addon objects
	type AddonType = { addon_project_id: number, addon_name: string, version: string }

	// Helper function to process addon categories
	const processCategory = (
		oldAddons: AddonType[],
		newAddons: AddonType[]
	) =>
	{
		// Find removed addons
		for (const oldAddon of oldAddons)
		{
			const found = newAddons.find((a) => a.addon_project_id === oldAddon.addon_project_id)
			if (found === undefined)
			{
				diff.removedAddons.push(oldAddon.addon_name)
			}
		}

		// Find updated addons
		for (const oldAddon of oldAddons)
		{
			const newAddon = newAddons.find((a) => a.addon_project_id === oldAddon.addon_project_id)
			if (newAddon !== undefined && oldAddon.version !== newAddon.version)
			{
				diff.updatedAddons.push({
					name: newAddon.addon_name,
					oldVersion: oldAddon.version,
					newVersion: newAddon.version
				})
			}
		}

		// Find new addons
		for (const newAddon of newAddons)
		{
			const found = oldAddons.find((a) => a.addon_project_id === newAddon.addon_project_id)
			if (found === undefined)
			{
				diff.newAddons.push(newAddon.addon_name)
			}
		}
	}	// Process each category
	processCategory(old.mods, newManifest.mods)
	processCategory(old.resourcepacks, newManifest.resourcepacks)
	processCategory(old.shaderpacks, newManifest.shaderpacks)
	processCategory(old.datapacks, newManifest.datapacks)

	return diff
})

const isUpdate = computed(() => manifestStore.previousManifest !== null)

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

async function loadExistingManifest(modpackPath: string)
{
	try
	{
		// Try to read existing manifest_old.json from the modpack directory first
		// This will be the previous version if an update has already been downloaded
		const oldManifestPath = `${modpackPath}/manifest_old.json`
		const oldManifestContent = await readFile(oldManifestPath)

		if (oldManifestContent !== null && oldManifestContent.trim().length > 0)
		{
			const oldManifest = JSON.parse(oldManifestContent)
			manifestStore.loadInstalledManifest(oldManifest)
			logger.info('Loaded existing manifest_old.json for update comparison', { oldManifestPath })
			return
		}
	}
	catch (err)
	{
		// manifest_old.json doesn't exist, try manifest.json
		logger.debug('No manifest_old.json found, trying manifest.json', { err })
	}

	try
	{
		// Fall back to reading current manifest.json
		const manifestPath = `${modpackPath}/manifest.json`
		const existingManifestContent = await readFile(manifestPath)

		if (existingManifestContent !== null && existingManifestContent.trim().length > 0)
		{
			const existingManifest = JSON.parse(existingManifestContent)
			manifestStore.loadInstalledManifest(existingManifest)
			logger.info('Loaded existing manifest.json for update comparison', { manifestPath })
		}
		else
		{
			manifestStore.loadInstalledManifest(null)
		}
	}
	catch (err)
	{
		// No existing manifest found or invalid format - this is fine for fresh installs
		logger.debug('No existing manifest found, proceeding with fresh install', { err })
		manifestStore.loadInstalledManifest(null)
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
			statusMessage.value = 'Writing files to disk...'

			// Check for existing manifest to use as previous manifest
			await loadExistingManifest(modpackPath)

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
