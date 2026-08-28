<template>
  <WorkspacePage heading="Install update">
    <template #lede>
      <template v-if="updateApplied">
        This update has been applied. Paste another code to install a new one.
      </template>
      <template v-else-if="manifest !== null">
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
        class="overflow-hidden rounded-box border bg-base-200 transition-colors duration-150 ease-(--ease-standard)"
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
            <span class="block text-xs font-medium tracking-wide text-base-content/60 uppercase">
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
            class="btn shrink-0 btn-sm"
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
            class="mb-1.5 block text-xs font-medium tracking-wide text-base-content/60 uppercase"
            for="user-update-code"
          >
            Update code
          </label>
          <div class="flex flex-wrap items-center gap-2">
            <label class="input min-w-0 flex-1 border-base-300 bg-base-100 font-mono text-sm input-md">
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
                :disabled="busy || manifest !== null"
                placeholder="paste update code…"
                spellcheck="false"
                autocomplete="off"
                @keydown.enter="handleFetch"
              />
            </label>

            <button
              v-if="manifest === null"
              type="button"
              class="btn gap-1.5 btn-md btn-primary"
              :disabled="!canFetch"
              @click="handleFetch"
            >
              <span
                v-if="downloading"
                class="loading loading-xs loading-spinner"
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
            class="mt-2 inline-block link text-xs text-base-content/50 link-hover"
          >
            from {{ appStore.githubRepo.length > 0 ? appStore.githubRepo : 'no repository set' }}
          </NuxtLink>
        </div>
      </div>
    </template>

    <!-- The content swaps between resting and the diff. That swap is a real
         state change, so it gets the one transition on this screen. Installing
         is deliberately NOT one of these states: the diff stays put and the
         work runs in a dialog over it. -->
    <Transition v-bind="paneTransition">
      <!-- The diff — the same screen, grown -->
      <div
        v-if="manifest !== null"
        class="space-y-4"
      >
        <div
          v-if="updateApplied"
          role="status"
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
              This is what was applied. Your modpack now matches it.
            </p>
          </div>
        </div>

        <UpdatePreview
          v-model:remove-extras="removeExtras"
          :added="addedRows"
          :updated="updatedRows"
          :removed="removedRows"
          :extras="extraRows"
          :unchanged="unchangedRows"
          :config-files="configFilesPreview"
          :custom-datapacks="customDatapacksPreview"
          :update-type="manifest.updateType"
          :applied="updateApplied"
          :locked="installing || updateApplied"
        />
      </div>

      <!-- Resting state -->
      <EmptyState
        v-else
        icon="mdi:package-variant-closed"
        title="Nothing to install yet"
      >
        Paste the code your admin sent you. You will see exactly what is added,
        updated and deleted before anything touches your modpack.
        <template #action>
          <NuxtLink
            to="/packs"
            class="btn gap-1.5 btn-sm"
          >
            <Icon
              name="mdi:view-grid-outline"
              size="1rem"
              aria-hidden="true"
            />
            Browse your packs
          </NuxtLink>
        </template>
      </EmptyState>
    </Transition>

    <InstallProgressDialog
      :open="installPhase !== 'idle'"
      :state="dialogFace"
      :progress="smoothProgress"
      :label="progressLabel"
      :error="installError"
      :summary="installSummary"
      @close="dismissInstallDialog"
    />

    <template #actions>
      <label
        v-if="showDiffActions && hasDestructiveChanges"
        class="flex cursor-pointer items-center gap-2.5 text-sm text-base-content/75"
      >
        <input
          v-model="acknowledged"
          type="checkbox"
          class="checkbox border-error checkbox-sm"
        />
        I understand {{ deletionCount }} {{ deletionCount === 1 ? 'file' : 'files' }}
        will be permanently deleted
      </label>

      <p
        v-else-if="showDiffActions"
        class="text-sm text-base-content/60"
      >
        Nothing will be deleted by this update.
      </p>

      <p
        v-else-if="updateApplied"
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
        v-else-if="updateApplied"
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
import type { AddonRow } from '~/components/domains/addons/AddonTable.vue'
import type { Addon, ConfigFileWithContent, Manifest, UpdateDiff } from '~/types'
import { ADDON_CATEGORIES } from '~/utils/addonCategories'

const { downloadFromGithub, downloadConfigFiles, installUpdate } = useUserApi()
const { getUpdateDiff } = useTauri()
const { trackOperation } = useInstallProgress()
const { notify } = useNotify()
const { paneTransition } = useMotion()
const manifestStore = useManifestStore()
const appStore = useAppStore()
const packsStore = usePacksStore()
const { $logger: logger } = useNuxtApp()

/**
 * Held on the manifest store, not locally, so the pack library can set the code
 * and the destination together and navigate straight here — this panel then
 * mounts already knowing which update it is showing.
 */
const uuid = computed({
	get: () => manifestStore.updateCode,
	set: (value: string) =>
	{
		manifestStore.updateCode = value
	}
})
const progress = ref(0)
/** Eased for display so event-driven jumps do not read as a broken bar. */
const { displayed: smoothProgress } = useSmoothProgress(progress)
/** Running commentary while work is in flight. Deliberately NOT a toast: the
 *  Tauri install path emits this repeatedly and would spam the corner. */
const progressMessage = ref('')
const downloading = ref(false)
const installing = ref(false)
const downloadedConfigFiles = ref<ConfigFileWithContent[]>([])
const downloadedConfigUpdateCode = ref<string | null>(null)
/** Immutable identity of the code that produced the visible preview. */
const fetchedUpdateCode = ref<string | null>(
	manifestStore.manifest === null ? null : manifestStore.updateCode.trim() || null
)
const acknowledged = ref(false)
const editingDestination = ref(false)

/**
 * The install dialog's own lifecycle, separate from `updateApplied` so that
 * dismissing the dialog does not discard the outcome it reported. `idle` means
 * no dialog; the other three are its three faces.
 */
const installPhase = ref<'idle' | 'running' | 'done' | 'failed'>('idle')
/**
 * The face the dialog wears, held one step behind `installPhase`.
 *
 * Deriving it straight from the phase meant dismissing flipped both props in
 * the same tick — open to false, and state back to `running` for want of a
 * fourth face. DaisyUI gives `.modal-box` a ~300ms close transition, so the
 * user pressed the button under "Update installed" and then watched
 * "Installing update", a progress bar and a disabled spinner fade out. Holding
 * the last real face means the dialog leaves saying what it said.
 */
const dialogFace = ref<'running' | 'done' | 'failed'>('running')
watch(installPhase, (phase) =>
{
	if (phase !== 'idle') dialogFace.value = phase
})
/** Shown inside the dialog. A toast would be behind the modal's own backdrop. */
const installError = ref<string | null>(null)
/** Survives dismissal: the diff on screen has been written to disk. */
const updateApplied = ref(false)

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
	() => manifest.value !== null && !installing.value && !updateApplied.value
)

/**
 * The diff is computed in Rust now, by the same function the installer uses to
 * decide what to delete, so this became asynchronous. It is held in a ref and
 * refreshed by the watcher below rather than derived synchronously.
 *
 * Null means "not known yet or failed", which is deliberately different from a
 * diff with no entries: a failed lookup must not render as "nothing changes"
 * next to an Apply button.
 */
const diff = ref<UpdateDiff | null>(null)

watch(
	[manifest, previousManifest],
	async ([newManifest, oldManifest]) =>
	{
		if (newManifest === null)
		{
			diff.value = null
			return
		}
		diff.value = await getUpdateDiff(oldManifest, newManifest)
	},
	{ immediate: true }
)

/**
 * Whether the player has asked CEMM to also remove the addons it did not
 * install. Off every time an update is fetched — see `resetFetchedState` — for
 * the same reason the destructive acknowledgement resets: a decision made about
 * one update is not a decision about the next one.
 */
const removeExtras = ref(false)

const previewData = computed(() =>
{
	if (manifest.value === null || diff.value === null) return null

	const hasChanges = previousManifest.value !== null && (
		diff.value.removed_addons.length > 0
		|| diff.value.updated_addon_ids.length > 0
		|| diff.value.new_addons.length > 0
	)

	return { diff: diff.value, hasChanges }
})

const hasAnyChange = computed(() => previewData.value?.hasChanges === true)

/** Everything this install will actually take off disk, given the opt-in. */
const deletionCount = computed(() =>
	removedRows.value.length + (removeExtras.value ? extraRows.value.length : 0)
)

const hasDestructiveChanges = computed(() => deletionCount.value > 0)

const previewMatchesSelection = computed(() =>
	manifest.value !== null
	&& manifestStore.belongsTo(appStore.modpackPath)
	&& fetchedUpdateCode.value !== null
	&& fetchedUpdateCode.value === uuid.value.trim()
)

const canApply = computed(() =>
	previewMatchesSelection.value
	&& appStore.modpackPath.trim().length > 0
	&& !installing.value
	&& !downloading.value
	&& (!hasDestructiveChanges.value || acknowledged.value)
)

/**
 * Every addon in the incoming manifest, paired with the category it was listed
 * under. The diff itself carries only names and project IDs, so this is where a
 * row learns whether it is a mod, a resource pack, a shader or a data pack —
 * which the preview needs, because it puts all four in one list.
 */
const incomingEntries = computed(() =>
	ADDON_CATEGORIES.flatMap((category) =>
		(manifest.value?.[category] ?? []).map((addon) => ({ addon, category }))
	)
)

/** The same, over the manifest being replaced. Deletions only exist here. */
const previousEntries = computed(() =>
	ADDON_CATEGORIES.flatMap((category) =>
		(previousManifest.value?.[category] ?? []).map((addon) => ({ addon, category }))
	)
)

/**
 * Keyed on project ID, which is unique across all four categories — the same
 * key `calculate_update_diff` decides deletions on.
 */
const previousById = computed(() =>
	new Map(previousEntries.value.map((entry) => [entry.addon.addon_project_id, entry]))
)

/**
 * Keyed on name, and only ever a fallback: names are not an identity. Two
 * CurseForge projects can share one, so a collision here would at worst label a
 * row with the wrong category or link to the wrong project page — never change
 * which addons the install deletes, which is keyed on project ID throughout.
 */
const previousByName = computed(() =>
	new Map(previousEntries.value.map((entry) => [entry.addon.addon_name, entry]))
)

/**
 * A link only counts when there is one behind it. `webSiteURL` is optional in
 * the manifest and arrives empty often enough that passing it straight through
 * put a hyperlinked name on rows that opened nothing.
 */
const projectUrlOf = (addon: Addon | undefined): string | undefined =>
	addon?.webSiteURL != null && addon.webSiteURL.length > 0 ? addon.webSiteURL : undefined

const addedRows = computed<AddonRow[]>(() =>
	(previewData.value?.diff.new_addons ?? []).map((name) =>
	{
		const entry = incomingEntries.value.find((candidate) => candidate.addon.addon_name === name)
		return {
			key: `new-${name}`,
			name,
			subtitle: entry?.addon.fileNameOnDisk ?? '',
			version: entry?.addon.version ?? '',
			versionNote: 'new install',
			tone: 'new' as const,
			label: 'New',
			category: entry?.category,
			thumbnailUrl: entry?.addon.thumbnailUrl,
			projectUrl: projectUrlOf(entry?.addon)
		}
	})
)

const updatedRows = computed<AddonRow[]>(() =>
	(previewData.value?.diff.updated_addon_ids ?? []).map((id) =>
	{
		const entry = incomingEntries.value.find((candidate) => candidate.addon.addon_project_id === id)
		const previous = previousById.value.get(id)
		return {
			key: `upd-${id}`,
			name: entry?.addon.addon_name ?? `Unknown addon (id ${id})`,
			subtitle: entry?.addon.fileNameOnDisk ?? '',
			version: entry?.addon.version ?? '',
			versionNote: previous !== undefined ? `from ${previous.addon.version}` : 'replaced',
			tone: 'updated' as const,
			// Imperative until it happens, past tense afterwards — the diff stays
			// on screen as the record of the install, so a row still saying
			// "Update" is describing something that is already done.
			label: updateApplied.value ? 'Updated' : 'Update',
			// An addon cannot change category between two manifests, so the old
			// one answers this just as well when the incoming lookup misses.
			category: entry?.category ?? previous?.category,
			thumbnailUrl: entry?.addon.thumbnailUrl,
			projectUrl: projectUrlOf(entry?.addon)
		}
	})
)

/**
 * Addons CEMM did not install: present on disk and known to CurseForge, but
 * absent from `cemm-manifest.json` — the player's own additions, and base-pack
 * addons the admin deliberately excluded from the upload. Resolved in Rust
 * alongside the baseline itself.
 */
const unmanagedIds = computed(() => new Set(manifestStore.unmanagedAddonIds))

/**
 * Everything in the pack that this update does not carry, before the question
 * of who is entitled to delete it.
 *
 * One pass rather than two, because the row and its provenance come from the
 * same entry: splitting the diff twice invited the two lists to disagree about
 * how many addons are leaving, on the one panel whose numbers have to be
 * trusted.
 */
const departingEntries = computed(() =>
{
	const ids = previewData.value?.diff.removed_addon_ids ?? []
	return (previewData.value?.diff.removed_addons ?? []).map((name, index) =>
	{
		// Iterating the names, not the IDs: the two arrays are filled entry for
		// entry by `calculate_update_diff`, and this list has to stay exactly as
		// long as the count the panel headline and the acknowledgement quote. The
		// ID is a better key for looking the addon up — so it is tried first —
		// but it never decides whether a row exists.
		const id = ids[index]
		const byId = id === undefined ? undefined : previousById.value.get(id)
		const previous = byId ?? previousByName.value.get(name)
		return {
			id,
			unmanaged: id !== undefined && unmanagedIds.value.has(id),
			name,
			index,
			previous
		}
	})
})

const removedRows = computed<AddonRow[]>(() =>
	departingEntries.value
		.filter((entry) => !entry.unmanaged)
		.map((entry) => ({
			key: `del-${entry.index}-${entry.id ?? entry.name}`,
			name: entry.name,
			subtitle: '',
			version: '',
			versionNote: 'removed from disk',
			tone: 'removed' as const,
			label: updateApplied.value ? 'Deleted' : 'Delete',
			struck: true,
			// Resolved against the manifest being replaced, since a removed addon
			// is by definition absent from the incoming one. This is the row where
			// "what is this thing?" is most worth answering: it lists what the
			// install is about to take off disk.
			category: entry.previous?.category,
			projectUrl: projectUrlOf(entry.previous?.addon)
		}))
)

/**
 * The same list for addons CEMM never installed. They are shown apart, and left
 * alone unless the player opts in: an update is the admin's statement about
 * their own pack, not a licence to remove a mod the player added themselves —
 * or one the admin excluded from the upload precisely to keep it local.
 */
const extraRows = computed<AddonRow[]>(() =>
	departingEntries.value
		.filter((entry) => entry.unmanaged)
		.map((entry) => ({
			key: `extra-${entry.index}-${entry.id ?? entry.name}`,
			name: entry.name,
			subtitle: '',
			version: entry.previous?.addon.version ?? '',
			versionNote: removeExtras.value ? 'removed from disk' : 'left in place',
			tone: removeExtras.value ? 'removed' as const : 'unchanged' as const,
			label: removeExtras.value
				? (updateApplied.value ? 'Deleted' : 'Delete')
				: 'Kept',
			struck: removeExtras.value,
			category: entry.previous?.category,
			projectUrl: projectUrlOf(entry.previous?.addon)
		}))
)

/** Project IDs of the addons in `extraRows`, for the baseline filter below. */
const extraIds = computed(() =>
	new Set(
		departingEntries.value
			.filter((entry) => entry.unmanaged && entry.id !== undefined)
			.map((entry) => entry.id as number)
	)
)

/**
 * The baseline the install actually runs against, which is what decides its
 * deletions: `calculate_update_diff` derives them from whatever manifest is
 * handed to it.
 *
 * Opting out is therefore expressed by withholding those addons from the
 * baseline rather than by asking the installer to make an exception — the
 * preview and the install then read the same document, and cannot disagree
 * about what is about to be deleted. Unmanaged addons the update *does* carry
 * stay in: dropping them would make the installer download files already on
 * disk.
 */
const installBaseline = computed<Manifest | null>(() =>
{
	const baseline = previousManifest.value
	if (baseline === null || removeExtras.value || extraIds.value.size === 0) return baseline

	const kept = (addons: Addon[]) => addons.filter((addon) => !extraIds.value.has(addon.addon_project_id))
	return {
		...baseline,
		mods: kept(baseline.mods),
		resourcepacks: kept(baseline.resourcepacks),
		shaderpacks: kept(baseline.shaderpacks),
		datapacks: kept(baseline.datapacks)
	}
})

const unchangedRows = computed<AddonRow[]>(() =>
{
	const changed = new Set([
		...addedRows.value.map((row) => row.name),
		...updatedRows.value.map((row) => row.name),
		...removedRows.value.map((row) => row.name)
	])
	return incomingEntries.value
		.filter((entry) => !changed.has(entry.addon.addon_name))
		.map(({ addon, category }) => ({
			key: `same-${addon.addon_project_id}-${addon.version}`,
			name: addon.addon_name,
			subtitle: addon.fileNameOnDisk,
			version: addon.version,
			versionNote: 'unchanged',
			tone: 'unchanged' as const,
			label: 'Same',
			category,
			thumbnailUrl: addon.thumbnailUrl,
			projectUrl: projectUrlOf(addon)
		}))
})

// Before confirmation, config file *content* hasn't been downloaded yet, so
// downloadedConfigFiles is empty and the preview showed nothing at all even
// though the manifest already lists which files are coming (F-P3-13). Falling
// back to manifest.config_files shows the file list immediately; the BIN/CFG
// badge only becomes available once content — and with it is_binary — exists.
//
// The download arrives flat, config files and custom data pack files in one
// list, because both are fetched the same way. The manifest is what separates
// them, so this reads the split from there rather than from the payload.
const configFilesPreview = computed(() =>
{
	const configPaths = new Set((manifest.value?.config_files ?? []).map((file) => file.relative_path))
	if (downloadedConfigFiles.value.length > 0)
	{
		return downloadedConfigFiles.value
			.filter((file) => configPaths.has(file.relative_path))
			.map((file) => ({
				relativePath: file.relative_path,
				badge: file.is_binary === true ? 'BIN' as const : 'CFG' as const
			}))
	}
	return (manifest.value?.config_files ?? []).map((file) => ({
		relativePath: file.relative_path,
		badge: null
	}))
})

/**
 * Data packs the update carries that did not come from CurseForge.
 *
 * Read straight off the manifest's own section: they are not addons and not
 * config files, and the point of that section is that a player sees a data pack
 * arriving as a data pack.
 */
const customDatapacksPreview = computed(() =>
	(manifest.value?.custom_datapacks ?? []).map((datapack) => ({
		name: datapack.name,
		archived: datapack.archived,
		fileCount: datapack.files.length
	}))
)

const progressLabel = computed(() =>
{
	if (progressMessage.value.length > 0) return progressMessage.value
	if (downloading.value) return 'Downloading from GitHub…'
	if (installing.value) return 'Installing addons and config files…'
	if (updateApplied.value) return 'Done'
	return 'Waiting to start'
})

/** The three figures the dialog reports back once the write has finished. */
const installSummary = computed(() => ({
	added: addedRows.value.length,
	updated: updatedRows.value.length,
	removed: removedRows.value.length
}))

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
	// The install dialog is a native modal and therefore in the top layer, so a
	// toast raised while it is open would be announced from behind its own
	// backdrop. Failures go into the dialog, which is where the user is looking;
	// its success wording is better than the callback's, so that one is dropped.
	if (installPhase.value !== 'idle' && (type === 'error' || type === 'success'))
	{
		if (type === 'error') installError.value = message
		return
	}
	notify(message, type)
}

const resetFetchedState = (clearUpdateCode: boolean) =>
{
	const preservedCode = uuid.value
	manifestStore.clearManifest()
	if (!clearUpdateCode) uuid.value = preservedCode
	fetchedUpdateCode.value = null
	acknowledged.value = false
	removeExtras.value = false
	downloadedConfigFiles.value = []
	downloadedConfigUpdateCode.value = null
	progress.value = 0
	updateApplied.value = false
	installPhase.value = 'idle'
	installError.value = null
	clearStatus()
}

const updateModpackPath = (newPath: string | string[] | null) =>
{
	const singlePath = Array.isArray(newPath) ? newPath[0] : newPath
	if (singlePath !== null && singlePath !== undefined && singlePath.trim().length > 0)
	{
		const invalidatesPreview = manifest.value !== null && !manifestStore.belongsTo(singlePath)
		appStore.modpackPath = singlePath
		logger.info({ path: singlePath }, 'Modpack path updated via PathSelector')
		if (invalidatesPreview)
		{
			resetFetchedState(false)
			setStatus('Destination changed. Fetch the update again to preview it against this folder.', 'info')
		}
	}
	else
	{
		const invalidatesPreview = manifest.value !== null
		appStore.modpackPath = ''
		if (invalidatesPreview) resetFetchedState(false)
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
	const requestedUpdateCode = uuid.value.trim()

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
			requestedUpdateCode,
			(value: number, message?: string) =>
			{
				progress.value = value
				if (message !== undefined) setStatus(message, 'info')
			},
			setStatus
		)
		if (result.success)
		{
			uuid.value = requestedUpdateCode
			fetchedUpdateCode.value = requestedUpdateCode
			progress.value = 100
			// Config payloads belong to exactly one fetched update. Invalidate the
			// previous payload only after the replacement manifest is available.
			downloadedConfigFiles.value = []
			downloadedConfigUpdateCode.value = null
			// Consent resets for every fetched update: acknowledging one diff is
			// not consent to a different one, and neither is agreeing to sweep up
			// addons CEMM did not install. The applied flag goes with them, or a
			// freshly fetched diff would inherit the previous one's "installed"
			// banner and claim to already be on disk.
			acknowledged.value = false
			removeExtras.value = false
			updateApplied.value = false
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
	resetFetchedState(true)
}

/**
 * Dismissal closes the dialog and nothing else. The outcome it reported lives
 * in `updateApplied`, so the screen behind it keeps the diff that was installed
 * rather than resetting out from under the user.
 */
const dismissInstallDialog = () =>
{
	installPhase.value = 'idle'
	clearStatus()
}

async function handleApply()
{
	if (!canApply.value) return
	installError.value = null
	installPhase.value = 'running'
	await confirmInstall()
}

async function confirmInstall()
{
	if (
		fetchedUpdateCode.value !== null
		&& downloadedConfigUpdateCode.value !== fetchedUpdateCode.value
		&& manifest.value !== null
		&& manifest.value.config_files.length > 0
	)
	{
		try
		{
			downloading.value = true
			progress.value = 0
			const result = await downloadConfigFiles(
				fetchedUpdateCode.value,
				manifest.value,
				(value: number, message?: string) =>
				{
					progress.value = value
					if (message !== undefined) setStatus(message, 'info')
				},
				setStatus
			)
			if (!result.success)
			{
				downloadedConfigFiles.value = []
				downloadedConfigUpdateCode.value = null
				return
			}

			downloadedConfigFiles.value = result.configFiles
			downloadedConfigUpdateCode.value = fetchedUpdateCode.value
			progress.value = 100
		}
		catch (error)
		{
			setStatus(
				`Could not download the config files: ${error instanceof Error ? error.message : 'unknown error'}`,
				'error'
			)
			installPhase.value = 'failed'
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
	// Pinned before the install starts rather than read from the refs inside the
	// callback below. The store's manifest can change while an install runs —
	// the user can navigate to another pack — and the install has to finish
	// against the manifest the preview was approved for, not whichever one is
	// current when the callback happens to fire.
	//
	// Guarding the const rather than `manifest.value` also keeps the non-null
	// narrowing, which TypeScript does not carry across a closure boundary when
	// it comes from a property access.
	const installingManifest = manifest.value
	const baseline = installBaseline.value
	const configFiles = downloadedConfigFiles.value

	if (installingManifest === null) return
	if (!previewMatchesSelection.value)
	{
		setStatus('The update preview no longer matches this destination or update code. Fetch it again before installing.', 'error')
		installPhase.value = 'failed'
		return
	}

	installing.value = true
	updateApplied.value = false
	progress.value = 0
	try
	{
		const installed = await trackOperation(
			(payload) =>
			{
				if (typeof payload.progress === 'number')
				{
					progress.value = payload.progress
				}
				if (typeof payload.message === 'string')
				{
					progressMessage.value = payload.message
				}
			},
			(operationId) => installUpdate(
				operationId,
				installingManifest,
				configFiles,
				baseline,
				(value: number, message?: string) =>
				{
					progress.value = value
					if (message !== undefined) setStatus(message, 'info')
				},
				setStatus
			)
		)
		updateApplied.value = installed
		installPhase.value = installed ? 'done' : 'failed'
		if (installed)
		{
			// The pack library sorts on this, and shows it as "Updated 3d ago".
			packsStore.recordInstalled(appStore.modpackPath)
		}
	}
	finally
	{
		installing.value = false
		if (updateApplied.value)
		{
			progress.value = 100
		}
		// A throw on the way in — setting up the progress listener, say — would
		// otherwise leave the dialog spinning on work that is not running.
		if (installPhase.value === 'running')
		{
			installPhase.value = 'failed'
		}
	}
}

const startOver = () =>
{
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
