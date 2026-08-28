<template>
  <WorkspacePage
    heading="Publish update"
    :fill-content="manifest !== null"
  >
    <template #lede>
      <template v-if="manifest === null">
        Load the modpack you have been modifying, then choose what your players receive.
      </template>
      <template v-else>
        Everything ships unless you switch it off.
      </template>
    </template>

    <!-- Instance context. Loading is an empty state, not a step, so once an
         instance is loaded its identity lives here permanently. -->
    <template
      v-if="manifest !== null"
      #context
    >
      <div class="rounded-box border border-base-300 bg-base-200 px-3.5 py-3">
        <div class="flex flex-wrap items-center gap-2">
          <span class="flex min-w-0 flex-1 items-center gap-2.5">
            <Icon
              name="mdi:folder-open-outline"
              size="1.05rem"
              class="shrink-0 text-primary"
              aria-hidden="true"
            />
            <span class="min-w-0">
              <span class="block truncate text-sm font-semibold">{{ instanceLabel }}</span>
              <span class="block truncate font-mono text-[0.6875rem] text-base-content/50">
                {{ totalAddons }} addons · {{ selectedConfigFiles.length }} config files
              </span>
            </span>
          </span>

          <button
            type="button"
            class="btn btn-sm"
            @click="handleLoadInstance"
          >
            Reload
          </button>
          <button
            type="button"
            class="btn btn-sm"
            @click="handleSaveManifest"
          >
            Save manifest
          </button>
        </div>

        <div class="mt-2.5 flex flex-wrap items-center gap-2 text-xs text-base-content/60">
          <span>Published as</span>
          <span class="inline-flex items-center gap-1.5 rounded-full border border-base-300 bg-base-100 px-2.5 py-1 font-mono text-[0.6875rem]">
            {{ customModpackName.trim().length > 0 ? customModpackName.trim() : 'no name set' }}
          </span>
          <button
            type="button"
            class="link cursor-pointer text-[0.6875rem] font-medium text-primary link-hover"
            :aria-expanded="editingName"
            @click="editingName = !editingName"
          >
            {{ editingName ? 'done' : 'rename' }}
          </button>
          <span class="text-base-content/40">— becomes the first half of the update code</span>
        </div>

        <div
          v-if="editingName"
          class="mt-3 border-t border-base-300 pt-3"
        >
          <label class="input w-full max-w-sm border-base-300 bg-base-100 font-mono text-xs input-sm">
            <Icon
              name="mdi:package-variant-closed"
              size="0.9375rem"
              class="shrink-0 text-base-content/40"
              aria-hidden="true"
            />
            <input
              id="admin-modpack-name"
              v-model="customModpackName"
              type="text"
              class="grow"
              placeholder="chillecke"
              aria-label="Modpack name"
              spellcheck="false"
              autocomplete="off"
            />
          </label>
        </div>
      </div>
    </template>

    <Transition v-bind="paneTransition">
      <!-- Resting state -->
      <EmptyState
        v-if="manifest === null"
        icon="mdi:folder-search-outline"
        title="No instance loaded"
      >
        Pick the modpack you have been modifying. CEMM reads its
        <span class="font-mono text-xs">minecraftinstance.json</span> and builds the
        list of addons you are running.
        <template #action>
          <NuxtLink
            to="/packs"
            class="btn gap-1.5 btn-primary btn-sm"
          >
            <Icon
              name="mdi:view-grid-outline"
              size="1rem"
              aria-hidden="true"
            />
            Browse your packs
          </NuxtLink>
          <button
            type="button"
            class="btn btn-sm"
            @click="handleLoadInstance"
          >
            Choose a file instead
          </button>
        </template>
      </EmptyState>

      <div
        v-else
        class="flex min-h-0 flex-1 flex-col gap-3"
      >
        <!-- Category pills, with config files as a peer rather than a panel
           stranded below a 300-row table. -->
        <FilterPills
          v-model="activePane"
          class="shrink-0"
          :options="panes"
          label="Content category"
          size="md"
        />

        <ConfigFilesSection
          v-if="activePane === 'config'"
          v-model="selectedConfigFiles"
          @status="handleStatus"
        />

        <AddonTable
          v-else
          :id="`admin-${activePane}`"
          :title="activeCategory.label"
          variant="manage"
          :rows="activeCategory.rows"
          :noun="activeCategory.label.toLowerCase()"
          :empty-label="`No ${activeCategory.label.toLowerCase()} in this instance.`"
          show-actions
          actions-label="Ships"
          fill
        >
          <template #actions>
            <button
              v-if="manualExcludedCount > 0"
              type="button"
              class="btn gap-1.5 btn-ghost btn-xs"
              @click="manifestStore.clearExclusions()"
            >
              <Icon
                name="mdi:restore"
                size="0.875rem"
                aria-hidden="true"
              />
              Include all
            </button>
          </template>

          <template #row-action="{ row }">
            <!-- A toggle, matching how CurseForge and Modrinth enable/disable a
               mod. Off means the addon stays on this machine and is left out of
               the upload. The wrapping label names the control per row. -->
            <label class="flex cursor-pointer items-center">
              <span class="sr-only">Include {{ row.name }} in the upload</span>
              <!-- Two lists behind one control. A custom data pack has no
                   CurseForge project, so it is not in the addon exclusion list
                   and cannot be keyed into it. -->
              <input
                type="checkbox"
                class="toggle rounded-full toggle-primary toggle-sm"
                :checked="isCustomRow(row) ? !isDatapackExcluded(row.name) : !manifestStore.isExcluded(row.name)"
                @change="isCustomRow(row) ? toggleDatapack(row.name) : manifestStore.toggleExclusion(row.name)"
              />
            </label>
          </template>
        </AddonTable>

        <p class="shrink-0 text-[0.8125rem] leading-relaxed text-base-content/55">
          Excluded addons stay installed on your machine — they are simply left out
          of what you publish. Use this for server-side or private mods.
          <template v-if="disabledExcludedCount > 0">
            {{ disabledNote }}
          </template>
        </p>
      </div>
    </Transition>

    <!-- Publishing is the action bar, and the resulting code lands here in
         place rather than on a separate screen. -->
    <template
      v-if="manifest !== null || selectedConfigFiles.length > 0"
      #actions
    >
      <template v-if="latestUpdateReference.length > 0">
        <span class="flex min-w-0 flex-1 items-center gap-2">
          <Icon
            name="mdi:check-circle-outline"
            size="1.05rem"
            class="shrink-0 text-success"
            aria-hidden="true"
          />
          <code class="min-w-0 flex-1 truncate rounded-field border border-base-300 bg-base-100 px-2.5 py-1.5 font-mono text-xs [user-select:all]">
            {{ latestUpdateReference }}
          </code>
          <button
            type="button"
            class="btn shrink-0 gap-1.5 btn-sm"
            @click="copyUpdateReference"
          >
            <Icon
              :name="copied ? 'mdi:check' : 'mdi:content-copy'"
              size="1rem"
              aria-hidden="true"
            />
            {{ copied ? 'Copied' : 'Copy code' }}
          </button>
        </span>
        <span
          class="sr-only"
          role="status"
        >{{ copyStatus }}</span>
      </template>

      <template v-else-if="uploading">
        <span class="min-w-0 flex-1">
          <span class="mb-1 flex items-baseline justify-between gap-3 text-xs">
            <span class="font-medium">Uploading to GitHub…</span>
            <span class="font-mono text-base-content/60 tabular-nums">{{ Math.round(smoothProgress) }}%</span>
          </span>
          <progress
            class="progress w-full"
            :value="smoothProgress"
            max="100"
            aria-label="Upload progress"
          />
        </span>
      </template>

      <p
        v-else
        class="text-sm text-base-content/60"
      >
        {{ shippingCount }} of {{ totalAddons }} addons ship<template v-if="excludedSummary.length > 0">
          · {{ excludedSummary }}
        </template><template v-if="selectedConfigFiles.length > 0">
          · {{ selectedConfigFiles.length }} config files
        </template>
      </p>

      <div class="flex-1" />

      <button
        type="button"
        class="btn gap-1.5 btn-primary btn-sm"
        :disabled="!canPublish"
        @click="handleUploadToGithub"
      >
        <span
          v-if="uploading"
          class="loading loading-xs loading-spinner"
          aria-hidden="true"
        />
        {{ latestUpdateReference.length > 0 ? 'Publish again' : 'Publish update' }}
      </button>
    </template>
  </WorkspacePage>
</template>

<script setup lang="ts">
import type { AddonRow } from '~/components/domains/addons/AddonTable.vue'
import type { Addon, ConfigFileWithContent, CustomDatapackWithContent } from '~/types'
import { ADDON_CATEGORIES, categoryLabel } from '~/utils/addonCategories'

const { loadInstance, saveManifest, uploadToGithub } = useAdminApi()
const { collectCustomDatapacks } = useTauri()
const { notify } = useNotify()
const { paneTransition } = useMotion()
const manifestStore = useManifestStore()
const packsStore = usePacksStore()
const { $logger: logger } = useNuxtApp()

const uploading = ref(false)
const progress = ref(0)
/** Eased for display so event-driven jumps do not read as a broken bar. */
const { displayed: smoothProgress } = useSmoothProgress(progress)
/** Running commentary while uploading. Not a toast: the upload path emits this
 *  repeatedly and would spam the corner. */
const progressMessage = ref('')
const selectedConfigFiles = ref<ConfigFileWithContent[]>([])

/**
 * Data packs in the instance that CurseForge did not install, grouped by pack.
 *
 * Held apart from `selectedConfigFiles` because they are not config files —
 * that was the whole complaint. They list under Data packs, beside the packs
 * CurseForge did install, and ship in the manifest's own `custom_datapacks`
 * section rather than smuggled into `config_files`.
 */
const customDatapacks = ref<CustomDatapackWithContent[]>([])

/**
 * Custom data packs the admin has switched off, by pack name.
 *
 * Deliberately not `manifestStore.excludedAddons`: that list is keyed on addon
 * names and is reset from the instance's own disabled addons, and a data pack
 * with no CurseForge project has no business in it.
 */
const excludedDatapacks = ref<string[]>([])

const isDatapackExcluded = (name: string) => excludedDatapacks.value.includes(name)

const toggleDatapack = (name: string) =>
{
	excludedDatapacks.value = isDatapackExcluded(name)
		? excludedDatapacks.value.filter((excluded) => excluded !== name)
		: [...excludedDatapacks.value, name]
}

/** What actually ships: everything the admin has not switched off. */
const shippingDatapacks = computed(() =>
	customDatapacks.value.filter((datapack) => !isDatapackExcluded(datapack.name))
)
const customModpackName = ref('')
const latestUpdateReference = ref('')
const copied = ref(false)
const copyStatus = ref('')
const editingName = ref(false)

/**
 * Where the loaded instance came from. Read off the manifest store rather than
 * held locally, because this panel is now sometimes mounted *after* the load —
 * the pack library loads a card and then navigates here, and a local ref would
 * be empty at that moment.
 */
const instanceDir = computed(() => manifestStore.sourcePath)

const activePane = ref('mods')

const manifest = computed(() => manifestStore.manifest)
const excludedCount = computed(() => manifestStore.excludedAddons.length)

/** Addons the loaded instance reports as switched off, by name. */
const disabledNames = computed(() =>
{
	const current = manifest.value
	if (current === null) return new Set<string>()
	return new Set(
		[current.mods, current.resourcepacks, current.shaderpacks, current.datapacks]
			.flat()
			.filter((addon) => addon.disabled === true)
			.map((addon) => addon.addon_name)
	)
})

/** Still switched off in CurseForge and still left out — the default state. */
const disabledExcludedCount = computed(
	() => manifestStore.excludedAddons.filter((name) => disabledNames.value.has(name)).length
)

/**
 * Exclusions the admin made themselves. "Include all" is offered against this
 * rather than the whole set, or it would be a live control on a screen where it
 * can do nothing — clearing exclusions never re-includes a disabled addon.
 */
const manualExcludedCount = computed(() => excludedCount.value - disabledExcludedCount.value)

const disabledNote = computed(() =>
{
	const count = disabledExcludedCount.value
	return count === 1
		? 'One addon is switched off in CurseForge, so it starts excluded — switch it back on here to publish it anyway.'
		: `${count} addons are switched off in CurseForge, so they start excluded — switch one back on here to publish it anyway.`
})

/** Action-bar tally. The disabled figure is stated as a subset, not an addend. */
const excludedSummary = computed(() =>
{
	if (excludedCount.value === 0) return ''
	if (disabledExcludedCount.value === 0) return `${excludedCount.value} excluded`
	return `${excludedCount.value} excluded (${disabledExcludedCount.value} disabled)`
})

const instanceLabel = computed(() =>
{
	const name = customModpackName.value.trim()
	if (name.length > 0) return name
	// Fall back to the folder the instance was loaded from, so the header can
	// actually identify which pack is about to be published.
	if (instanceDir.value.length > 0)
	{
		const parts = instanceDir.value.split(/[\\/]/).filter((part) => part.length > 0)
		if (parts.length > 0) return parts[parts.length - 1] as string
	}
	return 'Loaded instance'
})

/**
 * The `manage` variant of AddonTable renders no status chip, so everything a
 * row has to say about why it is or is not shipping has to land in the subtitle.
 * "Disabled" and "excluded" are separate facts and a row can carry both, so all
 * four combinations are spelled out rather than collapsed.
 */
const rowSubtitle = (addon: Addon, excluded: boolean): string =>
{
	if (addon.disabled === true)
	{
		return excluded
			? 'Disabled in CurseForge — not published'
			: 'Disabled in CurseForge — published anyway'
	}
	return excluded ? 'Excluded — stays on your machine' : addon.fileNameOnDisk
}

const toRows = (addons: Addon[]): AddonRow[] =>
	addons.map((addon) =>
	{
		const excluded = manifestStore.excludedAddons.includes(addon.addon_name)
		return {
			key: `${addon.addon_project_id}-${addon.version}`,
			name: addon.addon_name,
			subtitle: rowSubtitle(addon, excluded),
			version: addon.version,
			// Deliberately empty: the filename already sits under the project name,
			// and repeating it here made the version column carry no information.
			versionNote: '',
			tone: excluded ? 'excluded' as const : 'shipping' as const,
			label: excluded ? (addon.disabled === true ? 'Disabled' : 'Excluded') : 'Ships',
			struck: excluded,
			dimmed: excluded,
			thumbnailUrl: addon.thumbnailUrl,
			projectUrl: addon.webSiteURL ?? undefined
		}
	})

/**
 * Custom data packs as table rows, for the Data packs pane they belong in.
 *
 * Badged, because the two kinds sitting in one list have to stay tellable
 * apart: a CurseForge pack has a project page and a version behind it, and one
 * of these has neither — what it has is a file count and an origin outside
 * CurseForge.
 */
const customDatapackRows = computed<AddonRow[]>(() =>
	customDatapacks.value.map((datapack) =>
	{
		const excluded = isDatapackExcluded(datapack.name)
		const fileCount = datapack.files.length
		return {
			key: `custom-${datapack.name}`,
			name: datapack.name,
			subtitle: excluded
				? 'Excluded — stays in your instance'
				: (datapack.archived
					? 'Zipped data pack'
					: `Folder · ${fileCount} ${fileCount === 1 ? 'file' : 'files'}`),
			version: '',
			versionNote: '',
			badge: 'Custom',
			tone: excluded ? 'excluded' as const : 'shipping' as const,
			label: excluded ? 'Excluded' : 'Ships',
			struck: excluded,
			dimmed: excluded
		}
	})
)

const categories = computed(() =>
	ADDON_CATEGORIES.map((category) => ({
		key: category as string,
		label: categoryLabel(category),
		rows: category === 'datapacks'
			// CurseForge's own first, then the ones only CEMM knows about.
			? [...toRows(manifest.value?.datapacks ?? []), ...customDatapackRows.value]
			: toRows(manifest.value?.[category] ?? [])
	}))
)

/** Which rows in the Data packs pane the addon exclusion list does not govern. */
const customDatapackNames = computed(
	() => new Set(customDatapacks.value.map((datapack) => datapack.name))
)

const isCustomRow = (row: AddonRow) =>
	activePane.value === 'datapacks' && customDatapackNames.value.has(row.name)

const EMPTY_CATEGORY = { key: 'mods', label: categoryLabel('mods'), rows: [] as AddonRow[] }

const activeCategory = computed(
	() => categories.value.find((category) => category.key === activePane.value) ?? EMPTY_CATEGORY
)

/** Categories plus config files, which is a peer choice rather than a section. */
const panes = computed(() => [
	...categories.value.map((category) => ({
		value: category.key,
		label: category.label,
		count: category.rows.length
	})),
	{ value: 'config', label: 'Config files', count: selectedConfigFiles.value.length }
])

const totalAddons = computed(() =>
	categories.value.reduce((sum, category) => sum + category.rows.length, 0)
)

const shippingCount = computed(() => totalAddons.value - excludedCount.value)

const canPublish = computed(() =>
	!uploading.value
	&& (
		manifest.value !== null
		|| selectedConfigFiles.value.length > 0
		|| shippingDatapacks.value.length > 0
	)
)

const clearStatus = () =>
{
	progressMessage.value = ''
}

/**
 * In-flight informational messages stay inline beside the upload progress bar;
 * everything else is an outcome and becomes a toast.
 */
const setStatus = (message: string, type: 'success' | 'error' | 'info' | 'warning') =>
{
	if (type === 'info' && uploading.value)
	{
		progressMessage.value = message
		return
	}
	notify(message, type)
}

const handleStatus = (message: string, type: 'success' | 'error' | 'info' | 'warning') =>
{
	setStatus(message, type)
}

/**
 * Pick up the data packs CurseForge did not install, for whichever instance is
 * loaded — however it came to be loaded.
 *
 * Keyed on the loaded instance rather than run inside the load, because there
 * are two ways in and only one of them passes through this panel. The button
 * below loads through this component; the pack library loads from its own
 * screen and then navigates here, mounting this panel fresh afterwards.
 * Collecting inside `loadInstance` meant the library path threw the result away
 * and a custom data pack never reached an update published that way — which,
 * since CEMM opens on the pack library, is the path almost every publish takes.
 */
watch(instanceDir, async (directory) =>
{
	// Replaced, never merged: these describe the instance now loaded, and an
	// exclusion made against a different pack means nothing here.
	customDatapacks.value = []
	excludedDatapacks.value = []
	if (directory.trim().length === 0) return

	const collected = await collectCustomDatapacks(directory)
	if (!collected.ok)
	{
		logger.error({ error: collected.message, directory }, 'Failed to collect custom data packs')
		setStatus(`Custom data packs could not be read: ${collected.message}`, 'warning')
		return
	}

	customDatapacks.value = collected.value
}, { immediate: true })

async function handleLoadInstance()
{
	clearStatus()
	const result = await loadInstance(setStatus)
	if (result.success && typeof result.instanceDir === 'string')
	{
		packsStore.recordOpened(result.instanceDir)
	}
	if (manifest.value !== null)
	{
		// A freshly loaded instance invalidates the previous publish.
		latestUpdateReference.value = ''
		activePane.value = 'mods'
	}
}

async function handleSaveManifest()
{
	clearStatus()
	if (manifest.value !== null)
	{
		await saveManifest(manifest.value, selectedConfigFiles.value, shippingDatapacks.value, setStatus)
	}
}

async function handleUploadToGithub()
{
	if (
		manifest.value === null
		&& selectedConfigFiles.value.length === 0
		&& shippingDatapacks.value.length === 0
	)
	{
		return
	}

	clearStatus()
	latestUpdateReference.value = ''
	progress.value = 0
	uploading.value = true

	try
	{
		const result: { success: boolean, updateReference?: string } = await uploadToGithub(
			manifest.value,
			selectedConfigFiles.value,
			shippingDatapacks.value,
			customModpackName.value,
			(value: number, message?: string) =>
			{
				progress.value = value
				if (message !== undefined) setStatus(message, 'info')
			},
			setStatus
		)
		if (result.success && typeof result.updateReference === 'string')
		{
			latestUpdateReference.value = result.updateReference
			progress.value = 100
			// The pack library sorts on this, and shows it as "Published 3d ago".
			if (instanceDir.value.length > 0)
			{
				packsStore.recordPublished(instanceDir.value)
			}
		}
	}
	finally
	{
		uploading.value = false
	}
}

let copyTimer: ReturnType<typeof setTimeout> | null = null

async function copyUpdateReference()
{
	if (latestUpdateReference.value.length === 0) return

	try
	{
		await navigator.clipboard.writeText(latestUpdateReference.value)
		copied.value = true
		copyStatus.value = 'Update code copied to clipboard.'
	}
	catch (error)
	{
		logger.error({ error }, 'Clipboard write failed')
		// The code is select-all, so a clipboard failure is recoverable by hand.
		copyStatus.value = 'Could not copy automatically. Select the code and copy it manually.'
	}

	if (copyTimer !== null) clearTimeout(copyTimer)
	copyTimer = setTimeout(() =>
	{
		copied.value = false
	}, 2000)
}

onMounted(() =>
{
	logger.info('AdminPanel mounted')
})

onUnmounted(() =>
{
	if (copyTimer !== null) clearTimeout(copyTimer)
	logger.info('AdminPanel unmounted')
})
</script>
