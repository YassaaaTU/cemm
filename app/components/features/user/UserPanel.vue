<template>
  <div
    class="user-panel"
    role="main"
    aria-labelledby="user-mode-title"
  >
    <h1
      id="user-mode-title"
      class="sr-only"
    >
      User Mode
    </h1>

    <nav
      class="user-panel__progress-bar"
      aria-label="Setup progress"
    >
      <div
        v-for="(step, index) in wizardSteps"
        :key="step.key"
        class="user-panel__progress-item"
        :class="{
          'user-panel__progress-item--completed': stepCompleted[index],
          'user-panel__progress-item--active': expandedSections.includes(index),
        }"
      >
        <button
          class="btn btn-sm btn-ghost gap-1"
          :class="{ 'text-success': stepCompleted[index], 'text-primary': expandedSections.includes(index) }"
          @click="toggleSection(index)"
        >
          <Icon
            v-if="stepCompleted[index]"
            name="mdi:check-circle"
            size="1rem"
          />
          <Icon
            v-else
            :name="step.icon"
            size="1rem"
          />
          {{ step.label }}
        </button>
      </div>
    </nav>

    <!-- Source Section -->
    <div
      class="collapse collapse-arrow bg-base-200 mb-3"
      :class="{ 'collapse-open': expandedSections.includes(0) }"
    >
      <input
        type="checkbox"
        :checked="expandedSections.includes(0)"
        aria-label="Toggle update source section"
        @change="toggleSection(0)"
      />
      <div class="collapse-title font-semibold flex items-center gap-2">
        <Icon
          name="mdi:source-branch"
          size="1.2rem"
          class="text-secondary"
        />
        Update Source
        <span
          v-if="stepCompleted[0]"
          class="badge badge-success badge-sm"
        >
          Complete
        </span>
        <span
          v-else-if="!expandedSections.includes(0)"
          class="badge badge-ghost badge-sm"
        >
          Setup needed
        </span>
      </div>
      <div class="collapse-content">
        <div class="flex flex-col gap-3 pt-2">
          <label
            class="label"
            for="user-github-repository"
          >
            <span class="label-text">GitHub Repository</span>
          </label>
          <input
            id="user-github-repository"
            :value="githubRepo"
            type="text"
            class="input input-bordered w-full"
            :class="{ 'font-mono': true }"
            placeholder="owner/repository"
            aria-label="GitHub repository name"
            @input="githubRepo = ($event.target as HTMLInputElement).value"
            @blur="saveGithubRepo"
          />
          <label class="label">
            <span class="label-text-alt">Example: YassaaaTU/cemm-updates</span>
          </label>
        </div>
      </div>
    </div>

    <!-- Destination Section -->
    <div
      class="collapse collapse-arrow bg-base-200 mb-3"
      :class="{ 'collapse-open': expandedSections.includes(1) }"
    >
      <input
        type="checkbox"
        :checked="expandedSections.includes(1)"
        aria-label="Toggle destination section"
        @change="toggleSection(1)"
      />
      <div class="collapse-title font-semibold flex items-center gap-2">
        <Icon
          name="mdi:folder-marker"
          size="1.2rem"
          class="text-primary"
        />
        Destination
        <span
          v-if="stepCompleted[1]"
          class="badge badge-success badge-sm"
        >
          Complete
        </span>
        <span
          v-else-if="!expandedSections.includes(1)"
          class="badge badge-ghost badge-sm"
        >
          Setup needed
        </span>
      </div>
      <div class="collapse-content">
        <div class="flex flex-col gap-4 pt-2">
          <div>
            <label
              class="label"
              for="user-update-code"
            >
              <span class="label-text font-semibold">Modpack Directory</span>
            </label>
            <p class="text-xs opacity-70 mb-2">
              Select the modpack directory where you want to install the update
            </p>
            <PathSelector
              type="directory"
              title="Select Modpack Directory"
              :model-value="path"
              @update:model-value="updateModpackPath"
              @error="handlePathSelectorError"
            />
          </div>
          <div>
            <label class="label">
              <span class="label-text font-semibold">Update Code</span>
            </label>
            <input
              id="user-update-code"
              :value="uuid"
              type="text"
              class="input input-bordered w-full font-mono"
              placeholder="modpack-key/uuid"
              aria-label="Update code"
              @input="uuid = ($event.target as HTMLInputElement).value"
            />
            <label class="label">
              <span class="label-text-alt">Format: modpack-key/uuid or plain UUID (from your admin)</span>
            </label>
          </div>
          <button
            class="btn btn-primary btn-sm self-end"
            :disabled="!canDownload"
            @click="handleDownloadFromGithub"
          >
            <span
              v-if="downloading"
              class="loading loading-spinner loading-xs"
            />
            <Icon
              v-else
              name="mdi:download"
              size="1.1rem"
            />
            Download Manifest
          </button>
        </div>
      </div>
    </div>

    <!-- Preview Section -->
    <div
      class="collapse collapse-arrow bg-base-200 mb-3"
      :class="{ 'collapse-open': expandedSections.includes(2) }"
    >
      <input
        type="checkbox"
        :checked="expandedSections.includes(2)"
        aria-label="Toggle preview changes section"
        @change="toggleSection(2)"
      />
      <div class="collapse-title font-semibold flex items-center gap-2">
        <Icon
          name="mdi:eye"
          size="1.2rem"
          class="text-accent"
        />
        Preview Changes
        <span
          v-if="manifest"
          class="badge badge-info badge-sm"
        >
          {{ manifestTotalAddons }} addons
        </span>
        <span
          v-else
          class="badge badge-ghost badge-sm"
        >
          Download first
        </span>
      </div>
      <div class="collapse-content">
        <div
          v-if="manifest"
          class="pt-2 space-y-4"
        >
          <!-- Config-only notice -->
          <div
            v-if="manifest.updateType === 'config'"
            class="alert alert-info"
          >
            <Icon name="mdi:information" />
            <span>Config-only update: Only configuration files will be updated. No addons will be modified.</span>
          </div>

          <!-- Destructive changes warning -->
          <div
            v-if="hasDestructiveChanges"
            class="alert alert-warning"
          >
            <Icon name="mdi:alert" />
            <span>This update will remove {{ removedAddonNames.length }} addon(s) and update {{ updatedAddonNames.length }} addon(s). Old files will be deleted.</span>
          </div>

          <!-- New Addons List -->
          <div
            v-if="newAddonNames.length > 0"
            class="collapse collapse-arrow bg-base-100 border border-base-300"
          >
            <input
              type="checkbox"
              checked
              aria-label="Toggle new addons list"
            />
            <div class="collapse-title font-semibold text-success">
              <Icon
                name="mdi:plus-circle"
                size="1.1rem"
                class="mr-1"
              />
              New Addons ({{ newAddonNames.length }})
            </div>
            <div class="collapse-content">
              <ul class="menu bg-base-100 rounded-box">
                <li
                  v-for="name in newAddonNames"
                  :key="name"
                >
                  <div class="flex items-center gap-2">
                    <span class="badge badge-success badge-sm">NEW</span>
                    <span class="text-sm">{{ name }}</span>
                    <span
                      v-if="getNewAddonVersion(name)"
                      class="text-xs opacity-60 font-mono"
                    >
                      v{{ getNewAddonVersion(name) }}
                    </span>
                  </div>
                </li>
              </ul>
            </div>
          </div>

          <!-- Updated Addons List -->
          <div
            v-if="updatedAddonNames.length > 0"
            class="collapse collapse-arrow bg-base-100 border border-base-300"
          >
            <input
              type="checkbox"
              checked
              aria-label="Toggle updated addons list"
            />
            <div class="collapse-title font-semibold text-warning">
              <Icon
                name="mdi:update"
                size="1.1rem"
                class="mr-1"
              />
              Updated Addons ({{ updatedAddonNames.length }})
            </div>
            <div class="collapse-content">
              <ul class="menu bg-base-100 rounded-box">
                <li
                  v-for="name in updatedAddonNames"
                  :key="name"
                >
                  <div class="flex items-center gap-2">
                    <span class="badge badge-warning badge-sm">UPD</span>
                    <span class="text-sm">{{ name }}</span>
                  </div>
                </li>
              </ul>
            </div>
          </div>

          <!-- Removed Addons List -->
          <div
            v-if="removedAddonNames.length > 0"
            class="collapse collapse-arrow bg-base-100 border border-base-300"
          >
            <input
              type="checkbox"
              checked
              aria-label="Toggle removed addons list"
            />
            <div class="collapse-title font-semibold text-error">
              <Icon
                name="mdi:minus-circle"
                size="1.1rem"
                class="mr-1"
              />
              Removed Addons ({{ removedAddonNames.length }})
            </div>
            <div class="collapse-content">
              <ul class="menu bg-base-100 rounded-box">
                <li
                  v-for="name in removedAddonNames"
                  :key="name"
                >
                  <div class="flex items-center gap-2">
                    <span class="badge badge-error badge-sm">DEL</span>
                    <span class="text-sm line-through opacity-70">{{ name }}</span>
                  </div>
                </li>
              </ul>
            </div>
          </div>

          <!-- Config Files preview -->
          <div
            v-if="downloadedConfigFiles.length > 0"
            class="mt-2"
          >
            <h4 class="font-semibold text-sm mb-2">
              Config Files ({{ downloadedConfigFiles.length }})
            </h4>
            <div
              v-for="cf in downloadedConfigFiles"
              :key="cf.relative_path"
              class="flex items-center gap-2 py-1"
            >
              <span
                class="badge badge-sm"
                :class="cf.is_binary ? 'badge-secondary' : 'badge-primary'"
              >
                {{ cf.is_binary ? 'BIN' : 'CFG' }}
              </span>
              <span class="font-mono text-sm">{{ cf.relative_path }}</span>
            </div>
          </div>

          <!-- Unchanged Addons summary (collapsed by default) -->
          <div
            v-if="unchangedAddonNames.length > 0"
            class="collapse collapse-arrow bg-base-100 border border-base-300"
          >
            <input type="checkbox" />
            <div class="collapse-title font-semibold opacity-70">
              <Icon
                name="mdi:check-circle"
                size="1.1rem"
                class="mr-1"
              />
              Unchanged Addons ({{ unchangedAddonNames.length }})
            </div>
            <div class="collapse-content">
              <ul class="menu bg-base-100 rounded-box">
                <li
                  v-for="name in unchangedAddonNames"
                  :key="name"
                >
                  <div class="text-sm opacity-60">
                    {{ name }}
                  </div>
                </li>
              </ul>
            </div>
          </div>

          <!-- Apply Update button -->
          <div class="flex justify-end pt-2">
            <button
              class="btn btn-primary"
              :disabled="!canInstall"
              @click="showConfirmDialog = true; confirmAcknowledged = false"
            >
              {{ previewData?.hasChanges ? 'Apply Update' : 'Install' }}
            </button>
          </div>
        </div>

        <div
          v-else
          class="text-center py-8 opacity-60"
        >
          <Icon
            name="mdi:package-variant-closed"
            size="2.5rem"
            class="mb-2"
          />
          <p>No manifest downloaded yet</p>
        </div>
      </div>
    </div>

    <!-- Install Section -->
    <div
      class="collapse collapse-arrow bg-base-200 mb-3"
      :class="{ 'collapse-open': expandedSections.includes(3) }"
    >
      <input
        type="checkbox"
        :checked="expandedSections.includes(3)"
        aria-label="Toggle install section"
        @change="toggleSection(3)"
      />
      <div class="collapse-title font-semibold flex items-center gap-2">
        <Icon
          name="mdi:download"
          size="1.2rem"
          class="text-success"
        />
        Install
        <span
          v-if="installComplete"
          class="badge badge-success badge-sm"
        >
          Done
        </span>
      </div>
      <div class="collapse-content">
        <div class="flex flex-col gap-4 pt-2">
          <div class="flex items-center gap-3">
            <Icon
              name="mdi:progress-download"
              size="1.5rem"
              class="text-success"
            />
            <div>
              <h3 class="font-semibold">
                Installing Update
              </h3>
              <p
                v-if="installing"
                class="text-sm opacity-70"
              >
                Your modpack is being updated...
              </p>
              <p
                v-else-if="installComplete"
                class="text-sm opacity-70"
              >
                Update complete!
              </p>
            </div>
          </div>

          <ProgressBar
            :progress="progress"
            :label="getProgressLabel()"
            :color="getProgressColor()"
            :show-percentage="installing || downloading"
          />

          <div
            v-if="statusMessage"
            class="alert"
            :class="alertClass"
            role="alert"
          >
            <Icon :name="statusIcon" />
            <span>{{ statusMessage }}</span>
            <button
              class="btn btn-sm btn-ghost"
              aria-label="Dismiss status message"
              @click="clearStatus"
            >
              <Icon
                name="mdi:close"
                size="1.2rem"
              />
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Status Alert (outside sections) -->
    <div
      v-if="statusMessage && !expandedSections.includes(3)"
      class="alert mt-4"
      :class="alertClass"
      role="alert"
    >
      <Icon :name="statusIcon" />
      <span>{{ statusMessage }}</span>
      <button
        class="btn btn-sm btn-ghost"
        aria-label="Dismiss status message"
        @click="clearStatus"
      >
        <Icon
          name="mdi:close"
          size="1.2rem"
        />
      </button>
    </div>

    <!-- Confirmation Dialog -->
    <Teleport to="body">
      <div
        v-if="showConfirmDialog"
        class="modal modal-open z-50"
        role="dialog"
        aria-modal="true"
        aria-labelledby="confirm-update-title"
      >
        <div class="modal-box">
          <h3
            id="confirm-update-title"
            class="text-lg font-bold text-warning flex items-center gap-2"
          >
            <Icon
              name="mdi:alert-circle"
              size="1.4rem"
            />
            Confirm Update
          </h3>
          <p class="py-4">
            You are about to apply an update that will:
          </p>
          <ul class="menu bg-base-200 rounded-box mb-4">
            <li
              v-if="newAddonNames.length > 0"
              class="text-success"
            >
              <span>Add {{ newAddonNames.length }} new addon(s)</span>
            </li>
            <li
              v-if="updatedAddonNames.length > 0"
              class="text-warning"
            >
              <span>Update {{ updatedAddonNames.length }} addon(s)</span>
            </li>
            <li
              v-if="removedAddonNames.length > 0"
              class="text-error"
            >
              <span>Remove {{ removedAddonNames.length }} addon(s). These files will be deleted.</span>
            </li>
          </ul>
          <div
            v-if="hasDestructiveChanges"
            class="alert alert-warning mb-4"
          >
            <Icon name="mdi:alert" />
            <span>This action cannot be undone. Removed addon files will be permanently deleted.</span>
          </div>
          <div
            v-if="hasDestructiveChanges"
            class="form-control mb-4"
          >
            <label class="label cursor-pointer justify-start gap-2">
              <input
                v-model="confirmAcknowledged"
                type="checkbox"
                class="checkbox checkbox-sm"
              />
              <span class="label-text">I understand this action cannot be undone</span>
            </label>
          </div>
          <div class="modal-action">
            <button
              class="btn btn-ghost"
              @click="showConfirmDialog = false"
            >
              Cancel
            </button>
            <button
              class="btn btn-warning"
              :disabled="hasDestructiveChanges && !confirmAcknowledged"
              @click="handleConfirmedInstall"
            >
              <Icon
                name="mdi:check"
                size="1.1rem"
              />
              {{ hasDestructiveChanges ? 'I understand, apply update' : 'Apply update' }}
            </button>
          </div>
        </div>
        <div
          class="modal-backdrop"
          aria-hidden="true"
          @click="showConfirmDialog = false"
        />
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import { calculateUpdateDiff } from '~/composables/useTauri'
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

const uuid = ref('')
const progress = ref(0)
const statusMessage = ref('')
const statusType = ref<'success' | 'error' | 'info' | 'warning'>('info')
const downloading = ref(false)
const installing = ref(false)
const configFilesDownloaded = ref(false)
const downloadedConfigFiles = ref<ConfigFileWithContent[]>([])

const expandedSections = ref<number[]>([0])
const showConfirmDialog = ref(false)
const confirmAcknowledged = ref(false)
const installComplete = ref(false)

const wizardSteps = [
	{ key: 'source', label: 'Source', icon: 'mdi:source-branch' },
	{ key: 'destination', label: 'Destination', icon: 'mdi:folder-marker' },
	{ key: 'preview', label: 'Preview', icon: 'mdi:eye' },
	{ key: 'install', label: 'Install', icon: 'mdi:download' }
]

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

const githubRepoValid = computed(() =>
{
	const repo = githubRepo.value.trim()
	return repo.length > 0 && repo.includes('/')
})

const canDownload = computed(() =>
	!downloading.value
	&& uuid.value.trim().length > 0
	&& path.value.trim().length > 0
)

const canInstall = computed(() =>
	manifest.value !== null
	&& appStore.modpackPath.trim().length > 0
	&& !installing.value
	&& !downloading.value
)

const stepCompleted = computed(() =>
{
	const completed: Record<number, boolean> = {}
	completed[0] = githubRepoValid.value
	completed[1] = path.value.trim().length > 0 && uuid.value.trim().length > 0
	completed[2] = manifest.value !== null
	completed[3] = installComplete.value
	return completed
})

const previewData = computed(() =>
{
	if (manifest.value === null) return null

	const oldManifest = manifestStore.previousManifest
	const newManifest = manifest.value

	if (newManifest.updateType === 'config')
	{
		return {
			oldManifest,
			newManifest,
			diff: { removed_addons: [], updated_addon_ids: [], new_addons: [] },
			hasChanges: false,
			configFiles: downloadedConfigFiles.value
		}
	}

	const diff = calculateUpdateDiff(oldManifest, newManifest)

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

const hasDestructiveChanges = computed(() =>
	previewData.value !== null
	&& (previewData.value.diff.removed_addons.length > 0 || previewData.value.diff.updated_addon_ids.length > 0)
)

const allNewAddons = computed(() =>
{
	if (manifest.value === null) return []
	return [...manifest.value.mods, ...manifest.value.resourcepacks, ...manifest.value.shaderpacks, ...manifest.value.datapacks]
})

const newAddonNames = computed(() => previewData.value?.diff.new_addons ?? [])
const removedAddonNames = computed(() => previewData.value?.diff.removed_addons ?? [])
const updatedAddonNames = computed(() =>
{
	if (previewData.value === null) return []
	const ids = previewData.value.diff.updated_addon_ids
	return ids.map((id) =>
	{
		const addon = allNewAddons.value.find((a) => a.addon_project_id === id)
		return addon?.addon_name ?? `Unknown (ID: ${id})`
	})
})

const unchangedAddonNames = computed(() =>
{
	if (manifest.value === null) return []
	const changed = new Set([...newAddonNames.value, ...updatedAddonNames.value, ...removedAddonNames.value])
	return allNewAddons.value
		.filter((a) => !changed.has(a.addon_name))
		.map((a) => a.addon_name)
})

const getNewAddonVersion = (name: string): string =>
{
	const addon = allNewAddons.value.find((a) => a.addon_name === name)
	return addon?.version ?? ''
}

const manifestTotalAddons = computed(() => allNewAddons.value.length)

const statusIcon = computed(() =>
{
	const icons: Record<string, string> = {
		success: 'mdi:check-circle',
		error: 'mdi:alert-circle',
		warning: 'mdi:alert',
		info: 'mdi:information'
	}
	return icons[statusType.value] ?? icons.info
})

const alertClass = computed(() => `alert-${statusType.value}`)

const toggleSection = (index: number) =>
{
	const idx = expandedSections.value.indexOf(index)
	if (idx >= 0)
	{
		expandedSections.value.splice(idx, 1)
	}
	else
	{
		expandedSections.value.push(index)
	}
}

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
		if (!expandedSections.value.includes(2))
		{
			expandedSections.value.push(2)
		}
	}
	finally
	{
		downloading.value = false
		progress.value = 100
	}
}

async function confirmInstall()
{
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

	if (!expandedSections.value.includes(3))
	{
		expandedSections.value.push(3)
	}
	await performInstall()
}

async function handleConfirmedInstall()
{
	showConfirmDialog.value = false
	if (!expandedSections.value.includes(3))
	{
		expandedSections.value.push(3)
	}
	await confirmInstall()
}

async function performInstall()
{
	if (!canInstall.value || manifest.value === null) return

	installing.value = true
	installComplete.value = false
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

		const installed = await installUpdate(
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
		installComplete.value = installed
	}
	finally
	{
		installing.value = false
		if (installComplete.value)
		{
			progress.value = 100
		}
		if (typeof unlisten === 'function')
		{
			unlisten()
		}
	}
}

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

const route = useRoute()

const resetComponentState = () =>
{
	uuid.value = ''
	progress.value = 0
	statusMessage.value = ''
	statusType.value = 'info'
	downloading.value = false
	installing.value = false
	expandedSections.value = [0]
	configFilesDownloaded.value = false
	downloadedConfigFiles.value = []
	showConfirmDialog.value = false
	confirmAcknowledged.value = false
	installComplete.value = false
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

watch(manifest, (newManifest) =>
{
	if (newManifest !== null && !expandedSections.value.includes(2))
	{
		expandedSections.value.push(2)
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

<style scoped>
.user-panel {
	max-width: var(--container-lg);
	width: 100%;
	margin: 0 auto;
}

.user-panel__progress-bar {
	display: flex;
	align-items: center;
	gap: var(--space-2);
	flex-wrap: wrap;
	padding-bottom: var(--space-4);
}

.user-panel__progress-item {
	display: flex;
	align-items: center;
}
</style>
