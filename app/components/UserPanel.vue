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

    <!-- Quick GitHub Repo Input -->
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
        <p class="text-xs opacity-70 mb-4">
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
        for="modpack-path-selector"
        class="w-full label label-text"
      >
        Select the modpack directory where you want to install the update:
      </label>
      <path-selector
        id="modpack-path-selector"
        type="directory"
        title="Select Modpack Directory"
        :model-value="path"
        @update:model-value="updateModpackPath"
        @error="handlePathSelectorError"
      />
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
          @click="handleDownloadFromGithub"
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

    <div class="mt-6 flex flex-col gap-2">
      <progress-bar
        :progress="progress"
        :label="getProgressLabel()"
        :color="getProgressColor()"
        :show-percentage="downloading || installing"
      />
    </div>

    <!-- Status messages -->
    <app-alert
      v-if="statusMessage"
      class="mt-4"
      :message="statusMessage"
      :type="statusType"
      @close="clearStatus"
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

import type { ConfigFileWithContent } from '~/types'

interface InstallProgressEvent
{
	payload?: {
		progress?: number
		message?: string
	}
}

const { downloadFromGithub, downloadConfigFiles, installUpdate } = useUserApi()
const manifestStore = useManifestStore()
const appStore = useAppStore()
const { $logger: logger } = useNuxtApp()

// Component state
const uuid = ref('')
const progress = ref(0)
const statusMessage = ref('')
const statusType = ref<'success' | 'error' | 'info' | 'warning'>('info')
const downloading = ref(false)
const installing = ref(false)
const showPreview = ref(false)
const configFilesDownloaded = ref(false)
const downloadedConfigFiles = ref<ConfigFileWithContent[]>([])

// Computed properties
const manifest = computed(() => manifestStore.manifest)
const path = computed(() => appStore.modpackPath)
const previousManifest = computed(() => manifestStore.previousManifest)

const githubRepo = computed({
	get: () => appStore.githubRepo,
	set: (val: string) =>
	{
		appStore.githubRepo = val
	}
})

const canInstall = computed(() =>
	manifest.value !== null
	&& appStore.modpackPath.trim().length > 0
	&& !installing.value
	&& !downloading.value
)

// Preview data for UpdatePreview component
const previewData = computed(() =>
{
	if (manifest.value === null) return null

	const oldManifest = manifestStore.previousManifest
	const newManifest = manifest.value

	const diff = {
		removed_addons: [] as string[],
		updated_addon_ids: [] as number[],
		new_addons: [] as string[]
	}

	if (oldManifest !== null)
	{
		const processCategory = (
			oldAddons: Array<{ addon_project_id: number, addon_name: string, version: string, disabled?: boolean }>,
			newAddons: Array<{ addon_project_id: number, addon_name: string, version: string, disabled?: boolean }>
		) =>
		{
			for (const oldAddon of oldAddons)
			{
				if (oldAddon.disabled === true) continue
				const newAddon = newAddons.find((a) => a.addon_project_id === oldAddon.addon_project_id)
				if (newAddon === undefined || newAddon.disabled === true)
				{
					diff.removed_addons.push(oldAddon.addon_name)
				}
			}

			for (const oldAddon of oldAddons)
			{
				const newAddon = newAddons.find((a) => a.addon_project_id === oldAddon.addon_project_id)
				if (newAddon !== undefined && newAddon.disabled !== true && oldAddon.version !== newAddon.version)
				{
					diff.updated_addon_ids.push(oldAddon.addon_project_id)
				}
			}

			for (const newAddon of newAddons)
			{
				if (newAddon.disabled === true) continue
				const found = oldAddons.find((a) => a.addon_project_id === newAddon.addon_project_id)
				if (found === undefined)
				{
					diff.new_addons.push(newAddon.addon_name)
				}
			}
		}

		if (newManifest.updateType === 'config')
		{
			diff.removed_addons = []
			diff.updated_addon_ids = []
			diff.new_addons = []
		}
		else
		{
			processCategory(oldManifest.mods, newManifest.mods)
			processCategory(oldManifest.resourcepacks, newManifest.resourcepacks)
			processCategory(oldManifest.shaderpacks, newManifest.shaderpacks)
			processCategory(oldManifest.datapacks, newManifest.datapacks)
		}
	}
	else
	{
		if (newManifest.updateType !== 'config')
		{
			diff.new_addons = [
				...newManifest.mods.filter((addon) => addon.disabled !== true).map((addon) => addon.addon_name),
				...newManifest.resourcepacks.filter((addon) => addon.disabled !== true).map((addon) => addon.addon_name),
				...newManifest.shaderpacks.filter((addon) => addon.disabled !== true).map((addon) => addon.addon_name),
				...newManifest.datapacks.filter((addon) => addon.disabled !== true).map((addon) => addon.addon_name)
			]
		}
	}

	const hasChanges = oldManifest !== null && (
		diff.removed_addons.length > 0
		|| diff.updated_addon_ids.length > 0
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

// Status management
const clearStatus = () =>
{
	statusMessage.value = ''
	statusType.value = 'info'
}

const setStatus = (message: string, type: 'success' | 'error' | 'info' | 'warning') =>
{
	statusMessage.value = message
	statusType.value = type
}

// PathSelector event handlers
const updateModpackPath = (newPath: string | string[] | null) =>
{
	const singlePath = Array.isArray(newPath) ? newPath[0] : newPath
	if (singlePath !== null && singlePath !== undefined && singlePath.trim().length > 0)
	{
		appStore.modpackPath = singlePath
		logger.info('Modpack path updated via PathSelector', { path: singlePath })
	}
}

const handlePathSelectorError = (error: string) =>
{
	logger.error('PathSelector error', { error })
	setStatus(`Path selection error: ${error}`, 'error')
}

const saveGithubRepo = () =>
{
	if (githubRepo.value.trim())
	{
		logger.info('GitHub repository saved', { repo: githubRepo.value })
	}
}

// Action handlers
async function handleDownloadFromGithub()
{
	if (uuid.value.trim().length === 0)
	{
		setStatus('Please enter a valid UUID code.', 'error')
		return
	}

	clearStatus()
	progress.value = 0
	downloading.value = true

	try
	{
		await downloadFromGithub(
			uuid.value,
			(p: number, msg?: string) =>
			{
				progress.value = p
				if (msg !== undefined) setStatus(msg, 'info')
			},
			setStatus
		)
	}
	finally
	{
		downloading.value = false
		progress.value = 100
	}
}

async function confirmInstall()
{
	showPreview.value = false

	// Download config files if not already downloaded
	if (!configFilesDownloaded.value && uuid.value.trim().length > 0 && manifest.value !== null && manifest.value.config_files.length > 0)
	{
		try
		{
			downloading.value = true
			progress.value = 0
			const result = await downloadConfigFiles(
				uuid.value,
				manifest.value,
				(p: number, msg?: string) =>
				{
					progress.value = p
					if (msg !== undefined) setStatus(msg, 'info')
				},
				setStatus
			)
			downloadedConfigFiles.value = result.configFiles
			configFilesDownloaded.value = true
		}
		catch (error)
		{
			setStatus(`Failed to download config files: ${error instanceof Error ? error.message : 'Unknown error'}`, 'error')
			return
		}
		finally
		{
			downloading.value = false
			progress.value = 100
		}
	}

	await performInstall()
}

async function performInstall()
{
	if (!canInstall.value || manifest.value === null) return

	installing.value = true
	progress.value = 0
	let unlisten: UnlistenFn | null = null

	try
	{
		unlisten = await listen('install-progress', (event) =>
		{
			const prog = (event as InstallProgressEvent).payload?.progress
			const message = (event as InstallProgressEvent).payload?.message

			if (typeof prog === 'number')
			{
				progress.value = prog
			}
			if (typeof message === 'string')
			{
				statusMessage.value = message
			}
		})

		await installUpdate(
			manifest.value,
			downloadedConfigFiles.value,
			previousManifest.value,
			(p: number, msg?: string) =>
			{
				progress.value = p
				if (msg !== undefined) setStatus(msg, 'info')
			},
			setStatus
		)
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

// Accessibility helper methods
const getDownloadButtonDescription = () =>
{
	if (downloading.value || uuid.value.trim().length === 0)
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
	if (downloading.value) return 'Downloading from GitHub...'
	if (installing.value) return 'Installing addons and config files...'
	return ''
}

const getProgressColor = (): 'primary' | 'secondary' | 'accent' | 'success' | 'warning' | 'error' | 'info' =>
{
	if (downloading.value) return 'info'
	if (installing.value) return 'primary'
	if (progress.value === 100) return 'success'
	return 'primary'
}

// Navigation state management
const route = useRoute()

const resetComponentState = () =>
{
	uuid.value = ''
	progress.value = 0
	statusMessage.value = ''
	statusType.value = 'info'
	downloading.value = false
	installing.value = false
	showPreview.value = false
	configFilesDownloaded.value = false
	downloadedConfigFiles.value = []
	logger.info('Component state reset after navigation')
}

watch(() => route.name, (newRouteName, oldRouteName) =>
{
	if (oldRouteName === 'settings' && newRouteName === 'dashboard')
	{
		resetComponentState()
		logger.info('Detected navigation from settings to dashboard, state reset')
	}
})

onMounted(() =>
{
	logger.info('UserPanel mounted')
})

onUnmounted(() =>
{
	logger.info('UserPanel unmounted')
})
</script>
