<template>
  <div class="flex min-h-0 flex-1 flex-col">
    <!-- The wrapper exists because Nuxt page transitions require a single
         static root node, exactly as on the dashboard: a component root reads
         as possibly-multi-root and trips NUXT_E4004. -->
    <WorkspacePage
      heading="Your packs"
      fill-content
    >
      <template #lede>
        Your CurseForge library. Install an update into a pack, or publish one
        from it — whichever you pick is what CEMM switches to.
      </template>

      <template #context>
        <div class="flex flex-wrap items-center gap-x-3 gap-y-2 rounded-box border border-base-300 bg-base-200 px-3.5 py-3">
          <Icon
            :name="sourceIcon"
            size="1.05rem"
            class="shrink-0 text-primary"
            aria-hidden="true"
          />
          <span class="min-w-0 flex-1">
            <span class="block text-sm font-semibold">{{ sourceTitle }}</span>
            <span
              class="block truncate font-mono text-[0.6875rem] text-base-content/55"
              :title="packsStore.library?.instancesDir ?? ''"
            >{{ sourceDetail }}</span>
          </span>

          <button
            type="button"
            class="btn btn-sm"
            :disabled="packsStore.scanning"
            @click="packsStore.scan(true)"
          >
            <span
              v-if="packsStore.scanning"
              class="loading loading-xs loading-spinner"
              aria-hidden="true"
            />
            Refresh
          </button>
        </div>
      </template>

      <div class="flex min-h-0 flex-1 flex-col gap-3">
        <!--
          A scan can fail while CEMM still knows about packs from its own
          history. Saying "no library found" over a grid of them, with the
          footer counting them at the same time, was the screen contradicting
          itself — so the failure becomes a strip above the list it did not
          empty, and only speaks as a full empty state when there is genuinely
          nothing to show.
        -->
        <div
          v-if="libraryNotice !== null"
          role="alert"
          class="flex shrink-0 flex-wrap items-center gap-x-3 gap-y-2 rounded-box border border-warning/50 bg-warning/10 px-3.5 py-2.5"
        >
          <Icon
            name="mdi:alert-outline"
            size="1.05rem"
            class="shrink-0 text-warning"
            aria-hidden="true"
          />
          <span class="min-w-0 flex-1 text-[0.8125rem]">{{ libraryNotice }}</span>
          <button
            type="button"
            class="btn btn-sm"
            @click="chooseFolder"
          >
            Choose folder
          </button>
        </div>

        <!-- Above the list rather than under it. A pack CEMM could not read is
             a pack missing from the grid, and saying so at the foot of a
             thirty-six card scroll is saying it where nobody is looking. -->
        <p
          v-if="warningNote !== null"
          class="shrink-0 rounded-box border border-base-300 bg-base-200 px-3.5 py-2.5 text-[0.8125rem] leading-relaxed text-base-content/60"
        >
          {{ warningNote }}
        </p>

        <!-- Filters. Groups come from CurseForge's own groups.json, so the pills
             here are the same ones the user already organised their library with. -->
        <div
          v-if="packsStore.packs.length > 0"
          class="flex shrink-0 flex-wrap items-center gap-x-3 gap-y-2"
        >
          <div
            class="flex flex-wrap gap-1.5"
            role="group"
            aria-label="Filter packs"
          >
            <button
              v-for="option in filters"
              :key="option.value"
              type="button"
              class="cursor-pointer rounded-full border px-2.5 py-0.5 text-xs font-medium transition-colors duration-150 ease-(--ease-standard)"
              :class="activeFilter === option.value
                ? 'border-primary bg-primary/15 text-primary'
                : 'border-base-300 bg-base-200 text-base-content/60 hover:text-base-content'"
              :aria-pressed="activeFilter === option.value"
              @click="activeFilter = option.value"
            >
              {{ option.label }}
              <span class="ml-1 font-mono tabular-nums opacity-60">{{ option.count }}</span>
            </button>
          </div>

          <div class="flex-1" />

          <label class="input w-full max-w-68 border-base-300 bg-base-100 input-sm">
            <Icon
              name="mdi:magnify"
              size="1rem"
              class="shrink-0 text-base-content/40"
              aria-hidden="true"
            />
            <input
              v-model="search"
              type="search"
              class="grow text-sm"
              :placeholder="`Search ${packsStore.packs.length} packs…`"
              aria-label="Search packs"
            />
          </label>
        </div>

        <!--
          `relative` is load-bearing, not decoration. A statically positioned
          scroll container is clipped visually but its overflow is still counted
          by the ROOT scroller, so the window grew a second scrollbar that
          dragged the whole app up over blank space. Making it a containing
          block is what stops that; `overflow: hidden` on html/body does not.
        -->
        <div class="relative min-h-0 flex-1 overflow-y-auto">
          <!-- Scanning, first time only. A refresh keeps the current list on
               screen rather than blanking a grid the user is reading. -->
          <div
            v-if="packsStore.scanning && packsStore.library === null"
            class="grid grid-cols-[repeat(auto-fill,minmax(13.5rem,1fr))] gap-3"
          >
            <div
              v-for="n in 8"
              :key="n"
              class="h-33 animate-pulse rounded-box border border-base-300 bg-base-200"
            />
          </div>

          <!-- Not finding CurseForge is an ordinary outcome — it ships no
               official Linux build — so it gets a way forward, not an error. -->
          <EmptyState
            v-else-if="packsStore.packs.length === 0"
            :icon="emptyState.icon"
            :title="emptyState.title"
          >
            {{ emptyState.body }}
            <template #action>
              <button
                type="button"
                class="btn gap-1.5 btn-primary btn-sm"
                @click="chooseFolder"
              >
                <Icon
                  name="mdi:folder-open-outline"
                  size="1rem"
                  aria-hidden="true"
                />
                Choose instances folder
              </button>
            </template>
          </EmptyState>

          <p
            v-else-if="visiblePacks.length === 0"
            class="px-4 py-10 text-center text-sm text-base-content/50"
          >
            {{ noMatchMessage }}
          </p>

          <!-- No virtualisation, deliberately: the author's library is 36 packs
               and the scan behind it takes ~70ms warm. If someone turns up with
               hundreds this is the place to revisit. -->
          <div
            v-else
            class="grid grid-cols-[repeat(auto-fill,minmax(13.5rem,1fr))] gap-3 pb-1"
          >
            <PackCard
              v-for="pack in visiblePacks"
              :key="pack.instancePath"
              :pack="pack"
              :group-name="groupNameFor(pack.groupId)"
              :busy="busyFor(pack)"
              @install="installPack"
              @publish="publishPack"
              @forget="forgetPack"
            />
          </div>
        </div>
      </div>

      <template #actions>
        <p
          v-if="packsStore.packs.length > 0"
          class="text-sm text-base-content/60"
        >
          {{ visiblePacks.length }} of {{ packsStore.packs.length }} packs
        </p>
        <div class="flex-1" />
        <button
          type="button"
          class="btn btn-sm"
          @click="chooseFolder"
        >
          Choose a folder instead
        </button>
      </template>

      <PackUpdateDialog
        :pack="pendingPack"
        :busy="fetching"
        :error="fetchError"
        @submit="fetchUpdate"
        @close="closeDialog"
      />
    </WorkspacePage>
  </div>
</template>

<script setup lang="ts">
import type { AppMode } from '~/stores/app'
import type { PackRow } from '~/stores/packs'

definePageMeta({ layout: 'default' })

const appStore = useAppStore()
const packsStore = usePacksStore()
const manifestStore = useManifestStore()
const { notify } = useNotify()
const { loadInstance } = useAdminApi()
const { downloadFromGithub } = useUserApi()
const { selectDirectory } = useTauri()
const { $logger: logger } = useNuxtApp()

const search = ref('')
const activeFilter = ref('all')
/** Which card is working, and on what — only one action runs at a time. */
const busyPath = ref('')
const busyAction = ref<'install' | 'publish' | false>(false)
const pendingPack = ref<PackRow | null>(null)
const fetching = ref(false)
const fetchError = ref<string | null>(null)

/**
 * Re-stated as PackRow[] because the store reaches this file through Nuxt's
 * auto-import, where the setup store's inferred element type does not survive.
 * Everything below then filters and sorts against real types rather than any.
 */
const allPacks = computed<PackRow[]>(() => packsStore.packs)

const busyFor = (pack: PackRow): 'install' | 'publish' | false =>
	busyPath.value === pack.instancePath ? busyAction.value : false

const groupNameFor = (groupId: string | null): string | null =>
{
	if (groupId === null) return null
	return packsStore.groups.find((group) => group.id === groupId)?.name ?? null
}

const matchesFilter = (pack: PackRow): boolean =>
{
	if (activeFilter.value === 'all') return true
	if (activeFilter.value === 'recent') return pack.history !== null
	if (activeFilter.value === 'ungrouped') return pack.groupId === null
	return pack.groupId === activeFilter.value
}

const filters = computed(() =>
{
	const packs = allPacks.value
	const base = [
		{ value: 'all', label: 'All', count: packs.length },
		{
			value: 'recent',
			label: 'Used in CEMM',
			count: packs.filter((pack) => pack.history !== null).length
		}
	]
	const groups = packsStore.groups
		.map((group) => ({
			value: group.id,
			label: group.name,
			count: packs.filter((pack) => pack.groupId === group.id).length
		}))
		// A group whose packs are all elsewhere is a pill that can only ever
		// produce an empty grid.
		.filter((group) => group.count > 0)
	const ungrouped = packs.filter((pack) => pack.groupId === null).length

	return [
		...base,
		...groups,
		...(ungrouped > 0 && groups.length > 0
			? [{ value: 'ungrouped', label: 'Ungrouped', count: ungrouped }]
			: [])
	].filter((option) => option.value === 'all' || option.count > 0)
})

/** Free-text search covers everything a card actually shows. */
const matchesSearch = (pack: PackRow, term: string): boolean =>
	pack.name.toLowerCase().includes(term)
	|| pack.folderName.toLowerCase().includes(term)
	|| (pack.gameVersion ?? '').toLowerCase().includes(term)
	|| (pack.loader ?? '').toLowerCase().includes(term)
	|| (groupNameFor(pack.groupId) ?? '').toLowerCase().includes(term)

const visiblePacks = computed(() =>
{
	const term = search.value.trim().toLowerCase()
	return allPacks.value.filter(
		(pack) => matchesFilter(pack) && (term.length === 0 || matchesSearch(pack, term))
	)
})

const noMatchMessage = computed(() =>
	search.value.trim().length > 0
		? `Nothing matches “${search.value.trim()}”.`
		: 'Nothing matches that filter.'
)

/**
 * Shown as a strip above a list the failure did not empty. Null when there is
 * nothing wrong, or when there is nothing to show either — the empty state
 * covers that case on its own and would otherwise say the same thing twice.
 */
const libraryNotice = computed(() =>
{
	if (packsStore.packs.length === 0) return null
	if (packsStore.scanError !== null) return packsStore.scanError
	if (packsStore.library?.source === 'none')
	{
		return 'CEMM could not find your CurseForge library, so only packs it has used before are listed.'
	}
	return null
})

/** Instances the scan found but could not parse, so they are not in the grid. */
const warningNote = computed(() =>
{
	const warnings = packsStore.library?.warnings ?? []
	if (warnings.length === 0) return null
	const subject = warnings.length === 1
		? 'One folder could not be read, so it is not listed'
		: `${warnings.length} folders could not be read, so they are not listed`
	return `${subject}: ${warnings.join('; ')}`
})

const emptyState = computed(() =>
{
	if (packsStore.scanError !== null)
	{
		return {
			icon: 'mdi:alert-circle-outline',
			title: 'Could not read your packs',
			body: packsStore.scanError
		}
	}
	if (packsStore.library?.source === 'none')
	{
		return {
			icon: 'mdi:folder-search-outline',
			title: 'No CurseForge library found',
			body: 'CEMM looks for CurseForge\'s own settings to find your instances. Point it at the folder holding your modpacks instead.'
		}
	}
	return {
		icon: 'mdi:package-variant',
		title: 'No modpacks here',
		body: 'Nothing in that folder has a minecraftinstance.json in it.'
	}
})

const sourceIcon = computed(() =>
	packsStore.library?.source === 'curseforge' ? 'mdi:folder-star-outline' : 'mdi:folder-outline'
)

const sourceTitle = computed(() =>
{
	if (packsStore.library === null) return 'Looking for your packs…'
	if (packsStore.library.source === 'curseforge') return 'From your CurseForge library'
	if (packsStore.library.source === 'manual') return 'From the folder you chose'
	return 'No library found'
})

const sourceDetail = computed(() =>
	packsStore.library?.instancesDir ?? 'CurseForge settings could not be read'
)

/**
 * Manual selection is not a fallback bolted on for errors — it is the supported
 * path for anyone whose packs are not where CurseForge puts them, and the only
 * path on a machine without CurseForge at all.
 */
async function chooseFolder()
{
	const dir = await selectDirectory()
	if (dir === null || dir.trim().length === 0) return
	packsStore.setInstancesDirOverride(dir)
	await packsStore.scan(true)
}

/** The dialog reports the fetch with a spinner, so there is no bar to drive. */
const ignoreProgress = () =>
{
	// Intentionally empty.
}

function forgetPack(pack: PackRow)
{
	// Forgetting is only offered for a pack whose folder was checked for and is
	// not there, and the card only renders the control in that case. This is the
	// same rule stated twice on purpose: history is the sole record of a pack
	// outside the library, and a wrong instances folder must not be able to
	// discard it.
	if (pack.presence !== 'missing')
	{
		notify(
			`CEMM has not confirmed that ${pack.name} is gone, so it is keeping it.`,
			'warning'
		)
		return
	}
	packsStore.forget(pack.instancePath)
	notify(
		`Removed ${pack.name} from your packs.`,
		'success',
		`Nothing was found at ${pack.instancePath}. It will come back if you use it again.`
	)
}

/**
 * Drop a manifest that belongs somewhere else.
 *
 * Switching sides was the whole guard until now, and it was a serviceable
 * stand-in while a folder dialog inside a counter was the only way to change
 * packs. The library breaks that stand-in: you can pick a different pack
 * without changing sides, and the manifest left over from the last one would
 * then be diffed against this one — an install computing its deletions from a
 * pack that is not the destination. So the guard is the pack itself, and the
 * counter as well, because a diff fetched as a player is still not the admin's
 * working set.
 */
function releaseManifest(next: AppMode, instancePath: string)
{
	if (appStore.mode !== next || !manifestStore.belongsTo(instancePath))
	{
		manifestStore.clearManifest()
	}
}

function installPack(pack: PackRow)
{
	// Nothing is committed here. The dialog exists so the choice can still be
	// abandoned, and an admin who opened it to see what it does gets their
	// loaded instance back by cancelling — which is not true of the rail, where
	// the click itself is the commitment.
	fetchError.value = null
	pendingPack.value = pack
}

async function publishPack(pack: PackRow)
{
	busyPath.value = pack.instancePath
	busyAction.value = 'publish'
	try
	{
		// Released before the load rather than after it: loadInstance writes
		// into this same store and would otherwise diff the instance it is
		// reading against whatever the previous pack left behind.
		releaseManifest('admin', pack.instancePath)
		const result = await loadInstance(
			(message, type) =>
			{
				// The outcome is the navigation itself; only failures need saying.
				if (type === 'error' || type === 'warning') notify(message, type)
			},
			pack.instanceFile
		)
		if (!result.success)
		{
			return
		}
		// The counter moves only once there is something on it to move to. A
		// pack deleted inside the scan's 30-second window still renders its
		// buttons, and a failed load that had already switched sides left the
		// user on a counter they never reached, with nothing loaded.
		appStore.setMode('admin')
		packsStore.recordOpened(pack.instancePath)
		await navigateTo('/dashboard')
	}
	finally
	{
		busyPath.value = ''
		busyAction.value = false
	}
}

async function fetchUpdate(code: string)
{
	const pack = pendingPack.value
	if (pack === null) return

	fetching.value = true
	fetchError.value = null
	// Held on an object rather than straight into the ref: the status callback
	// runs inside downloadFromGithub, which TypeScript's control flow cannot
	// see, so a ref written only there still reads as its initial null.
	const failure = { message: '' }
	try
	{
		// This, not the dialog opening, is where the choice is committed — so
		// this is where the counter moves and the previous pack's manifest is
		// dropped. Both happen before the destination changes, so a diff can
		// never be generated for one pack while another one is still loaded.
		releaseManifest('user', pack.instancePath)
		appStore.setMode('user')
		// The destination is set before the fetch, because generating the diff
		// baseline reads the pack that is about to be updated. clearManifest
		// resets updateCode, so the code is written after the release, not before.
		appStore.modpackPath = pack.instancePath
		manifestStore.updateCode = code

		const result = await downloadFromGithub(
			code,
			ignoreProgress,
			(message, type) =>
			{
				if (type === 'error') failure.message = message
			}
		)

		if (!result.success)
		{
			fetchError.value = failure.message.length > 0
				? failure.message
				: 'That code did not return an update. Check it and try again.'
			return
		}

		packsStore.recordOpened(pack.instancePath)
		pendingPack.value = null
		await navigateTo('/dashboard')
	}
	catch (error)
	{
		logger.error({ error }, 'Pack library fetch failed')
		fetchError.value = error instanceof Error ? error.message : 'Could not fetch that update.'
	}
	finally
	{
		fetching.value = false
	}
}

const closeDialog = () =>
{
	if (fetching.value) return
	pendingPack.value = null
	fetchError.value = null
}

onMounted(() =>
{
	void packsStore.scan()
})
</script>
