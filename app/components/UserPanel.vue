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
    <!-- Install Preview Modal -->
    <dialog
      v-if="showPreview"
      open
      class="modal modal-open"
    >
      <div class="modal-box max-w-2xl">
        <h3 class="font-bold text-lg mb-2">
          Install Preview
        </h3>
        <p class="mb-2">
          The following will be installed to <span class="font-mono">{{ appStore.modpackPath }}</span>:
        </p>
        <div class="mb-2">
          <strong>Mods:</strong>
          <ul class="list-disc ml-6">
            <li
              v-for="mod in manifest?.mods ?? []"
              :key="mod.addon_project_id"
            >
              {{ mod.addon_name }} ({{ mod.version }})
            </li>
          </ul>
        </div>
        <div class="mb-2">
          <strong>Resourcepacks:</strong>
          <ul class="list-disc ml-6">
            <li
              v-for="rp in manifest?.resourcepacks ?? []"
              :key="rp.addon_project_id"
            >
              {{ rp.addon_name }} ({{ rp.version }})
            </li>
          </ul>
        </div>
        <div class="mb-2">
          <strong>Shaderpacks:</strong>
          <ul class="list-disc ml-6">
            <li
              v-for="sp in manifest?.shaderpacks ?? []"
              :key="sp.addon_project_id"
            >
              {{ sp.addon_name }} ({{ sp.version }})
            </li>
          </ul>
        </div>
        <div class="mb-2">
          <strong>Datapacks:</strong>
          <ul class="list-disc ml-6">
            <li
              v-for="dp in manifest?.datapacks ?? []"
              :key="dp.addon_project_id"
            >
              {{ dp.addon_name }} ({{ dp.version }})
            </li>
          </ul>
        </div>
        <div class="mb-2">
          <strong>Config files:</strong>
          <ul class="list-disc ml-6">
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
            Confirm Install
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
import { invoke } from '@tauri-apps/api/core'
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

const canInstall = computed(() =>
	manifest.value !== null
	&& appStore.modpackPath.trim().length > 0
	&& !installing.value
	&& !downloading.value
)

const showPreview = ref(false)

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

			// Prepare files for batch write
			const filesToWrite: Array<[string, string]> = []

			// Add manifest.json
			filesToWrite.push(['manifest.json', JSON.stringify(result.manifest, null, 2)])

			// Add config files
			for (const configFile of result.configFiles)
			{
				filesToWrite.push([configFile.path, configFile.content])
			}

			// Write all files to modpack directory
			const writeSuccess = await writeFile(modpackPath, filesToWrite)
			if (!writeSuccess)
			{
				statusMessage.value = 'Download successful, but failed to write files to disk.'
				statusType.value = 'warning'
				return
			}

			statusMessage.value = `Download successful! ${filesToWrite.length} files written to ${modpackPath}`
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

		await invoke('install_update', {
			modpackPath: appStore.modpackPath,
			manifest: manifest.value,
			configFiles: downloadedConfigFiles.value
		})
		statusMessage.value = 'Installation complete!'
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
