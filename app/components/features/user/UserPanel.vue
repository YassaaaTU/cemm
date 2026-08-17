<template>
  <WorkspacePage heading="Install update">
    <template #lede>
      <template v-if="manifest !== null">
        Nothing is written to your modpack until you confirm.
      </template>
      <template v-else>
        Paste a code to see exactly what it changes.
      </template>
    </template>

    <!-- The only per-use input. The repository and the modpack folder are
         settings, not steps, so they sit here as context rather than as
         screens the user walks through on every update. -->
    <template #context>
      <div
        class="overflow-hidden rounded-box border bg-base-200 transition-colors duration-150 ease-[var(--ease-standard)]"
        :class="manifest !== null ? 'border-primary/50' : 'border-base-300'"
      >
        <!-- Destination first, and stated in full. This is the folder that gets
             written to and deleted from, so it cannot be a footnote. Showing
             only the folder name was actively unsafe: a library holding
             "FTB Evolution" and "FTB Evolution (1)" renders two identical
             chips for two different modpacks. -->
        <div class="flex flex-wrap items-start gap-x-4 gap-y-2 border-b border-base-300 bg-base-300/40 px-4 py-3">
          <span class="grid size-9 shrink-0 place-items-center rounded-box bg-base-100 text-base-content/60">
            <Icon
              name="mdi:folder-outline"
              size="1.125rem"
              aria-hidden="true"
            />
          </span>

          <span class="min-w-0 flex-1">
            <span class="block text-xs font-medium uppercase tracking-wide text-base-content/45">
              Installing to
            </span>
            <span
              v-if="appStore.modpackPath.length > 0"
              class="mt-0.5 block truncate text-[0.9375rem] font-semibold"
              :title="appStore.modpackPath"
            >{{ modpackLabel }}</span>
            <span
              v-else
              class="mt-0.5 block text-[0.9375rem] font-semibold text-warning"
            >No modpack folder chosen</span>
            <span
              v-if="appStore.modpackPath.length > 0"
              class="mt-0.5 block truncate font-mono text-xs text-base-content/50"
              :title="appStore.modpackPath"
            >{{ appStore.modpackPath }}</span>
          </span>

          <button
            type="button"
            class="btn btn-sm shrink-0"
            :aria-expanded="editingDestination"
            @click="editingDestination = !editingDestination"
          >
            {{ editingDestination ? 'Done' : 'Change' }}
          </button>
        </div>

        <div
          v-if="editingDestination"
          class="border-b border-base-300 px-4 py-3"
        >
          <PathSelector
            type="directory"
            title="Select modpack directory"
            hint="The folder containing your modpack — the one with a mods folder inside it."
            :model-value="appStore.modpackPath"
            @update:model-value="updateModpackPath"
            @error="handlePathSelectorError"
          />
        </div>

        <!-- Then the code — the only thing that changes per update. -->
        <div class="px-4 py-3">
          <label
            class="mb-1.5 block text-xs font-medium uppercase tracking-wide text-base-content/45"
            for="user-update-code"
          >
            Update code
          </label>
          <div class="flex flex-wrap items-center gap-2">
            <label class="input input-md min-w-0 flex-1 border-base-300 bg-base-100 font-mono text-sm">
              <Icon
                name="mdi:key-variant"
                size="1rem"
                class="shrink-0"
                :class="manifest !== null ? 'text-primary' : 'text-base-content/40'"
                aria-hidden="true"
              />
              <input
                id="user-update-code"
                v-model="uuid"
                type="text"
                class="grow"
                placeholder="paste update code…"
                spellcheck="false"
                autocomplete="off"
                @keydown.enter="handleFetch"
              />
            </label>

            <button
              v-if="manifest === null"
              type="button"
              class="btn btn-primary btn-md gap-1.5"
              :disabled="!canFetch"
              @click="handleFetch"
            >
              <span
                v-if="downloading"
                class="loading loading-spinner loading-xs"
                aria-hidden="true"
              />
              {{ downloading ? 'Fetching…' : 'Fetch' }}
            </button>
            <button
              v-else
              type="button"
              class="btn btn-md"
              :disabled="installing"
              @click="clearFetched"
            >
              Clear
            </button>
          </div>

          <NuxtLink
            to="/settings"
            class="link link-hover mt-2 inline-block text-xs text-base-content/50"
          >
            from {{ appStore.githubRepo.length > 0 ? appStore.githubRepo : 'no repository set' }}
          </NuxtLink>
        </div>
      </div>
    </template>

    <!-- The content swaps between resting, diff and installing. That swap is a
         real state change, so it gets the one transition on this screen. -->
    <Transition v-bind="paneTransition">
      <!-- Installing / installed -->
      <div
        v-if="installing || installComplete"
        class="max-w-xl space-y-4"
      >
        <div
          v-if="installComplete"
          class="flex items-start gap-3 rounded-box border border-success/50 bg-success/10 px-4 py-3.5"
        >
          <Icon
            name="mdi:check-circle-outline"
            size="1.25rem"
            class="mt-px shrink-0 text-success"
            aria-hidden="true"
          />
          <div>
            <p class="text-sm font-semibold text-success">
              Update installed
            </p>
            <p class="mt-0.5 text-xs text-base-content/65">
              Your modpack now matches the update. You can launch the game.
            </p>
          </div>
        </div>

        <div v-else>
          <div class="mb-1.5 flex items-baseline justify-between gap-3">
            <p class="text-sm font-medium">
              {{ progressLabel }}
            </p>
            <p class="font-mono text-xs tabular-nums text-base-content/60">
              {{ Math.round(smoothProgress) }}%
            </p>
          </div>
          <progress
            class="progress w-full"
            :value="smoothProgress"
            max="100"
            :aria-label="progressLabel"
          />
        </div>
      </div>

      <!-- The diff — the same screen, grown -->
      <UpdatePreview
        v-else-if="manifest !== null"
        :added="addedRows"
        :updated="updatedRows"
        :removed="removedRows"
        :unchanged="unchangedRows"
        :config-files="configFilesPreview"
        :update-type="manifest.updateType"
      />

      <!-- Resting state -->
      <EmptyState
        v-else
        icon="mdi:package-variant-closed"
        title="Nothing to install yet"
      >
        Paste the code your admin sent you. You will see exactly what is added,
        updated and deleted before anything touches your modpack.
      </EmptyState>
    </Transition>

    <template #actions>
      <label
        v-if="showDiffActions && hasDestructiveChanges"
        class="flex cursor-pointer items-center gap-2.5 text-sm text-base-content/75"
      >
        <input
          v-model="acknowledged"
          type="checkbox"
          class="checkbox checkbox-sm border-error"
        />
        I understand {{ removedRows.length }} {{ removedRows.length === 1 ? 'file' : 'files' }}
        will be permanently deleted
      </label>

      <p
        v-else-if="showDiffActions"
        class="text-sm text-base-content/60"
      >
        Nothing will be deleted by this update.
      </p>

      <p
        v-else-if="installComplete"
        class="text-sm text-base-content/60"
      >
        Done — you can close CEMM or install another update.
      </p>

      <div class="flex-1" />

      <!-- The destination at the moment of commit. The context bar scrolls
           away with the content, so this is where "which modpack" has to be
           answered when the irreversible button is pressed. -->
      <span
        v-if="showDiffActions && appStore.modpackPath.length > 0"
        class="flex min-w-0 items-center gap-1.5 text-xs text-base-content/55"
        :title="appStore.modpackPath"
      >
        <Icon
          name="mdi:folder-outline"
          size="0.875rem"
          class="shrink-0"
          aria-hidden="true"
        />
        <span class="max-w-[18rem] truncate">{{ modpackLabel }}</span>
      </span>

      <button
        v-if="showDiffActions"
        type="button"
        class="btn btn-sm"
        :class="hasDestructiveChanges ? 'btn-error' : 'btn-primary'"
        :disabled="!canApply"
        @click="handleApply"
      >
        {{ hasAnyChange ? 'Install update' : 'Install' }}
      </button>

      <button
        v-else-if="installComplete"
        type="button"
        class="btn btn-primary btn-sm"
        @click="startOver"
      >
        Install another
      </button>
    </template>
  </WorkspacePage>
</template>

<script setup lang="ts">
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import type { AddonRow } from '~/components/domains/addons/AddonTable.vue'
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
const { notify } = useNotify()
const { paneTransition } = useMotion()
const manifestStore = useManifestStore()
const appStore = useAppStore()
const { $logger: logger } = useNuxtApp()

const uuid = ref('')
const progress = ref(0)
/** Eased for display so event-driven jumps do not read as a broken bar. */
const { displayed: smoothProgress } = useSmoothProgress(progress)
/** Running commentary while work is in flight. Deliberately NOT a toast: the
 *  Tauri install path emits this repeatedly and would spam the corner. */
const progressMessage = ref('')
const downloading = ref(false)
const installing = ref(false)
const configFilesDownloaded = ref(false)
const downloadedConfigFiles = ref<ConfigFileWithContent[]>([])
const installComplete = ref(false)
const acknowledged = ref(false)
const editingDestination = ref(false)

const manifest = computed(() => manifestStore.manifest)
const previousManifest = computed(() => manifestStore.previousManifest)

/** Just the folder name — the full path is in the title attribute. */
const modpackLabel = computed(() =>
{
	const path = appStore.modpackPath.trim()
	if (path.length === 0) return 'no folder chosen'
	const parts = path.split(/[\\/]/).filter((part) => part.length > 0)
	return parts[parts.length - 1] ?? path
})

/**
 * Deliberately does NOT require a modpack folder. Gating the button on it left
 * the user with a dead control and no stated reason; handleFetch now explains
 * the missing folder and opens the picker instead.
 */
const canFetch = computed(() => !downloading.value && uuid.value.trim().length > 0)

/** The diff is on screen and actionable. */
const showDiffActions = computed(
	() => manifest.value !== null && !installing.value && !installComplete.value
)

const previewData = computed(() =>
{
	if (manifest.value === null) return null

	const oldManifest = previousManifest.value
	const newManifest = manifest.value

	// calculateUpdateDiff keys config-only behavior to the explicit updateType
	// discriminator, so the preview and installer agree without preventing a
	// legitimate full update from emptying an addon category.
	const diff = calculateUpdateDiff(oldManifest, newManifest)

	const hasChanges = oldManifest !== null && (
		diff.removed_addons.length > 0
		|| diff.updated_addon_ids.length > 0
		|| diff.new_addons.length > 0
	)

	return { diff, hasChanges: oldManifest === null ? false : hasChanges }
})

const hasAnyChange = computed(() => previewData.value?.hasChanges === true)

const hasDestructiveChanges = computed(() =>
	previewData.value !== null && previewData.value.diff.removed_addons.length > 0
)

const canApply = computed(() =>
	manifest.value !== null
	&& appStore.modpackPath.trim().length > 0
	&& !installing.value
	&& !downloading.value
	&& (!hasDestructiveChanges.value || acknowledged.value)
)

const allIncomingAddons = computed(() =>
{
	if (manifest.value === null) return []
	return [
		...manifest.value.mods,
		...manifest.value.resourcepacks,
		...manifest.value.shaderpacks,
		...manifest.value.datapacks
	]
})

const previousById = computed(() =>
{
	const map = new Map<number, string>()
	const previous = previousManifest.value
	if (previous === null) return map
	for (const addon of [...previous.mods, ...previous.resourcepacks, ...previous.shaderpacks, ...previous.datapacks])
	{
		map.set(addon.addon_project_id, addon.version)
	}
	return map
})

const addedRows = computed<AddonRow[]>(() =>
	(previewData.value?.diff.new_addons ?? []).map((name) =>
	{
		const addon = allIncomingAddons.value.find((candidate) => candidate.addon_name === name)
		return {
			key: `new-${name}`,
			name,
			subtitle: addon?.fileNameOnDisk ?? '',
			version: addon?.version ?? '',
			versionNote: 'new install',
			tone: 'new' as const,
			label: 'New',
			thumbnailUrl: addon?.thumbnailUrl
		}
	})
)

const updatedRows = computed<AddonRow[]>(() =>
	(previewData.value?.diff.updated_addon_ids ?? []).map((id) =>
	{
		const addon = allIncomingAddons.value.find((candidate) => candidate.addon_project_id === id)
		const from = previousById.value.get(id)
		return {
			key: `upd-${id}`,
			name: addon?.addon_name ?? `Unknown addon (id ${id})`,
			subtitle: addon?.fileNameOnDisk ?? '',
			version: addon?.version ?? '',
			versionNote: from !== undefined ? `from ${from}` : 'replaced',
			tone: 'updated' as const,
			label: 'Update',
			thumbnailUrl: addon?.thumbnailUrl
		}
	})
)

const removedRows = computed<AddonRow[]>(() =>
	(previewData.value?.diff.removed_addons ?? []).map((name) => ({
		key: `del-${name}`,
		name,
		subtitle: '',
		version: '',
		versionNote: 'removed from disk',
		tone: 'removed' as const,
		label: 'Delete',
		struck: true
	}))
)

const unchangedRows = computed<AddonRow[]>(() =>
{
	if (manifest.value === null) return []
	const changed = new Set([
		...addedRows.value.map((row) => row.name),
		...updatedRows.value.map((row) => row.name),
		...removedRows.value.map((row) => row.name)
	])
	return allIncomingAddons.value
		.filter((addon) => !changed.has(addon.addon_name))
		.map((addon) => ({
			key: `same-${addon.addon_project_id}-${addon.version}`,
			name: addon.addon_name,
			subtitle: addon.fileNameOnDisk,
			version: addon.version,
			versionNote: 'unchanged',
			tone: 'unchanged' as const,
			label: 'Same',
			thumbnailUrl: addon.thumbnailUrl
		}))
})

// Before confirmation, config file *content* hasn't been downloaded yet, so
// downloadedConfigFiles is empty and the preview showed nothing at all even
// though the manifest already lists which files are coming (F-P3-13). Falling
// back to manifest.config_files shows the file list immediately; the BIN/CFG
// badge only becomes available once content — and with it is_binary — exists.
const configFilesPreview = computed(() =>
{
	if (downloadedConfigFiles.value.length > 0)
	{
		return downloadedConfigFiles.value.map((file) => ({
			relativePath: file.relative_path,
			badge: file.is_binary === true ? 'BIN' as const : 'CFG' as const
		}))
	}
	return (manifest.value?.config_files ?? []).map((file) => ({
		relativePath: file.relative_path,
		badge: null
	}))
})

const progressLabel = computed(() =>
{
	if (progressMessage.value.length > 0) return progressMessage.value
	if (downloading.value) return 'Downloading from GitHub…'
	if (installing.value) return 'Installing addons and config files…'
	if (installComplete.value) return 'Done'
	return 'Waiting to start'
})

const clearStatus = () =>
{
	progressMessage.value = ''
}

const busy = computed(() => downloading.value || installing.value)

/**
 * Routes the shared status callback: in-flight informational messages stay
 * inline beside the progress bar, everything else is an outcome and becomes a
 * toast.
 */
const setStatus = (message: string, type: 'success' | 'error' | 'info' | 'warning') =>
{
	if (type === 'info' && busy.value)
	{
		progressMessage.value = message
		return
	}
	notify(message, type)
}

const updateModpackPath = (newPath: string | string[] | null) =>
{
	const singlePath = Array.isArray(newPath) ? newPath[0] : newPath
	if (singlePath !== null && singlePath !== undefined && singlePath.trim().length > 0)
	{
		appStore.modpackPath = singlePath
		logger.info({ path: singlePath }, 'Modpack path updated via PathSelector')
	}
	else
	{
		appStore.modpackPath = ''
	}
}

const handlePathSelectorError = (error: string) =>
{
	logger.error({ error }, 'PathSelector error')
	setStatus(`Could not use that folder: ${error}`, 'error')
}

async function handleFetch()
{
	if (!canFetch.value) return

	if (appStore.modpackPath.trim().length === 0)
	{
		notify('Choose the modpack folder to install into first.', 'error')
		editingDestination.value = true
		return
	}

	clearStatus()
	progress.value = 0
	downloading.value = true

	try
	{
		const result = await downloadFromGithub(
			uuid.value,
			(value: number, message?: string) =>
			{
				progress.value = value
				if (message !== undefined) setStatus(message, 'info')
			},
			setStatus
		)
		if (result.success)
		{
			progress.value = 100
			// Consent resets for every fetched update: acknowledging one diff is
			// not consent to a different one.
			acknowledged.value = false
			editingDestination.value = false
		}
	}
	finally
	{
		downloading.value = false
	}
}

const clearFetched = () =>
{
	manifestStore.clearManifest()
	uuid.value = ''
	acknowledged.value = false
	configFilesDownloaded.value = false
	downloadedConfigFiles.value = []
	progress.value = 0
	clearStatus()
}

async function handleApply()
{
	if (!canApply.value) return
	await confirmInstall()
}

async function confirmInstall()
{
	if (
		!configFilesDownloaded.value
		&& uuid.value.trim().length > 0
		&& manifest.value !== null
		&& manifest.value.config_files.length > 0
	)
	{
		try
		{
			downloading.value = true
			progress.value = 0
			const result = await downloadConfigFiles(
				uuid.value,
				manifest.value,
				(value: number, message?: string) =>
				{
					progress.value = value
					if (message !== undefined) setStatus(message, 'info')
				},
				setStatus
			)
			downloadedConfigFiles.value = result.configFiles
			configFilesDownloaded.value = true
			if (result.success)
			{
				progress.value = 100
			}
		}
		catch (error)
		{
			setStatus(
				`Could not download the config files: ${error instanceof Error ? error.message : 'unknown error'}`,
				'error'
			)
			return
		}
		finally
		{
			downloading.value = false
		}
	}

	await performInstall()
}

async function performInstall()
{
	if (manifest.value === null) return

	installing.value = true
	installComplete.value = false
	progress.value = 0
	let unlisten: UnlistenFn | null = null

	try
	{
		unlisten = await listen('install-progress', (event) =>
		{
			const value = (event as InstallProgressEvent).payload?.progress
			const message = (event as InstallProgressEvent).payload?.message

			if (typeof value === 'number')
			{
				progress.value = value
			}
			if (typeof message === 'string')
			{
				progressMessage.value = message
			}
		})

		const installed = await installUpdate(
			manifest.value,
			downloadedConfigFiles.value,
			previousManifest.value,
			(value: number, message?: string) =>
			{
				progress.value = value
				if (message !== undefined) setStatus(message, 'info')
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

const startOver = () =>
{
	installComplete.value = false
	clearFetched()
}

onMounted(() =>
{
	logger.info('UserPanel mounted')
})

onUnmounted(() =>
{
	logger.info('UserPanel unmounted')
})
</script>
