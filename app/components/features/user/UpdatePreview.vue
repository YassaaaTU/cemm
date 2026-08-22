<template>
  <div class="space-y-4">
    <!-- Four figures before any list: this is the decision. -->
    <div class="grid grid-cols-2 gap-2.5 sm:grid-cols-4">
      <div
        v-for="tally in tallies"
        :key="tally.label"
        class="rounded-box border border-base-300 bg-base-200 px-3.5 py-3"
      >
        <p class="text-xs text-base-content/50">
          {{ tally.label }}
        </p>
        <p
          class="mt-1.5 font-mono text-2xl leading-none font-bold tabular-nums"
          :class="tally.count > 0 ? tally.tone : 'text-base-content/25'"
        >
          {{ tally.count }}
        </p>
      </div>
    </div>

    <div
      v-if="updateType === 'config'"
      role="alert"
      class="alert alert-soft text-sm alert-info"
    >
      <Icon
        name="mdi:information-outline"
        size="1.1rem"
        aria-hidden="true"
      />
      <span v-if="applied">
        <strong>Config-only update.</strong>
        Only configuration files changed — none of your addons were touched.
      </span>
      <span v-else>
        <strong>Config-only update.</strong>
        Only configuration files change — none of your addons will be touched.
      </span>
    </div>

    <!-- Deletions lead and stay open. This is the only part of an install that
         destroys something the user already has, so it is never something they
         have to go looking for. -->
    <div
      v-if="removed.length > 0"
      class="overflow-hidden rounded-box border border-error/60"
    >
      <div class="flex items-start gap-2.5 bg-error/10 px-3.5 py-3">
        <span class="grid size-6 shrink-0 place-items-center rounded-md bg-error/20 text-error">
          <Icon
            name="mdi:alert-outline"
            size="0.875rem"
            aria-hidden="true"
          />
        </span>
        <div class="min-w-0">
          <p class="text-sm font-semibold text-error">
            {{ deletionHeadline }}
          </p>
          <!-- Stated in prose as well as in the pills, so the make-up of the
               deletion is on screen before anyone touches a control. -->
          <p
            v-if="deletionBreakdown.length > 0"
            class="mt-0.5 text-xs font-medium text-base-content/80"
          >
            {{ deletionBreakdown }}
          </p>
          <p class="mt-0.5 text-xs text-base-content/65">
            {{ deletionNote }}
          </p>
        </div>
      </div>

      <AddonTable
        id="preview-removed"
        :title="applied ? 'Deleted' : 'Being deleted'"
        :rows="visibleRemoved"
        :total="removed.length"
        :max-height="200"
        noun="addons"
        show-type
        empty-label="No addons of that type in this deletion."
      >
        <!-- A type filter here narrows, it never conceals: it starts on All,
             the header keeps the full total beside the visible count, and the
             breakdown above names every category regardless of it. -->
        <template
          v-if="removedTypeFilters.length > 2"
          #filters
        >
          <FilterPills
            v-model="removedType"
            :options="removedTypeFilters"
            label="Filter deletions by addon type"
          />
        </template>
      </AddonTable>
    </div>

    <AddonTable
      v-if="allIncoming.length > 0"
      id="preview-incoming"
      :title="applied ? 'Installed' : 'Incoming'"
      :rows="incoming"
      :total="allIncoming.length"
      :max-height="300"
      noun="addons"
      show-type
      empty-label="No addons match both filters."
    >
      <template #filters>
        <div class="flex flex-wrap items-center gap-x-3 gap-y-2">
          <FilterPills
            v-if="incomingTypeFilters.length > 2"
            v-model="incomingType"
            :options="incomingTypeFilters"
            label="Filter incoming addons by addon type"
          />
          <span
            v-if="incomingTypeFilters.length > 2"
            class="h-4 w-px bg-base-300"
            aria-hidden="true"
          />
          <FilterPills
            v-model="activeFilter"
            :options="changeFilters"
            label="Filter incoming addons by change"
          />
        </div>
      </template>
    </AddonTable>

    <!-- Config files, listed by path — that is how a player recognises whether
         a change touches something they customised. -->
    <div
      v-if="configFiles.length > 0"
      class="overflow-hidden rounded-box border border-base-300 bg-base-200"
    >
      <div class="flex items-center gap-3 border-b border-base-300 px-3 py-2.5">
        <h3 class="text-sm font-semibold">
          Config files
        </h3>
        <span class="font-mono text-xs text-base-content/50 tabular-nums">{{ configFiles.length }}</span>
      </div>
      <ul class="max-h-40 overflow-y-auto">
        <li
          v-for="file in configFiles"
          :key="file.relativePath"
          class="flex items-center gap-2 border-b border-base-300/40 px-3 py-1.5 last:border-b-0"
        >
          <StatusChip
            v-if="file.badge !== null"
            tone="unchanged"
            :label="file.badge"
          />
          <span class="min-w-0 truncate font-mono text-xs text-base-content/75">{{ file.relativePath }}</span>
        </li>
      </ul>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { AddonRow } from '~/components/domains/addons/AddonTable.vue'
import { ADDON_CATEGORIES, type AddonCategory, categoryCountPhrase, categoryLabel, joinPhrases } from '~/utils/addonCategories'

const props = withDefaults(
	defineProps<{
		added: AddonRow[]
		updated: AddonRow[]
		removed: AddonRow[]
		unchanged: AddonRow[]
		configFiles: Array<{ relativePath: string, badge: 'BIN' | 'CFG' | null }>
		updateType?: 'full' | 'config'
		/**
		 * The same diff, after it has been written to disk. The screen stays up
		 * as the record of what was installed, so every sentence on it that
		 * described something about to happen has to stop claiming that.
		 */
		applied?: boolean
	}>(),
	{ updateType: undefined, applied: false }
)

const deletionHeadline = computed(() =>
{
	const noun = props.removed.length === 1 ? 'addon' : 'addons'
	const verb = props.removed.length === 1
		? (props.applied ? 'was deleted' : 'will be deleted')
		: (props.applied ? 'were deleted' : 'will be deleted')
	return `${props.removed.length} ${noun} ${verb} from your modpack`
})

const deletionNote = computed(() =>
{
	const single = props.removed.length === 1
	if (props.applied)
	{
		return single
			? 'That file was removed from disk permanently.'
			: 'Those files were removed from disk permanently.'
	}
	return single
		? 'That file is removed from disk permanently. This cannot be undone.'
		: 'Those files are removed from disk permanently. This cannot be undone.'
})

type ChangeKey = 'all' | 'new' | 'updated' | 'same'
/** `all` plus the four manifest categories. */
type TypeKey = 'all' | AddonCategory

const activeFilter = ref<ChangeKey>('all')
const incomingType = ref<TypeKey>('all')
const removedType = ref<TypeKey>('all')

const ofType = (rows: AddonRow[], type: TypeKey): AddonRow[] =>
	type === 'all' ? rows : rows.filter((row) => row.category === type)

const countOf = (rows: AddonRow[], type: TypeKey): number => ofType(rows, type).length

/** Which categories a set actually contains, in manifest order. */
const typesIn = (rows: AddonRow[]): AddonCategory[] =>
	ADDON_CATEGORIES.filter((category) => rows.some((row) => row.category === category))

const tallies = computed(() => [
	{ label: 'Added', count: props.added.length, tone: 'text-success' },
	{ label: 'Updated', count: props.updated.length, tone: 'text-info' },
	{ label: 'Deleted', count: props.removed.length, tone: 'text-error' },
	{ label: 'Untouched', count: props.unchanged.length, tone: 'text-base-content/70' }
])

const allIncoming = computed<AddonRow[]>(() => [...props.added, ...props.updated, ...props.unchanged])

/**
 * Added, updated and unchanged share one list rather than three stacked tables,
 * and are narrowed on two independent axes: what the update does to an addon,
 * and what kind of addon it is. Deletions are deliberately NOT in here — they
 * keep their own panel above, with its own type filter, where a change filter
 * can never reach them.
 */
const incomingByChange = computed<AddonRow[]>(() =>
{
	if (activeFilter.value === 'new') return props.added
	if (activeFilter.value === 'updated') return props.updated
	if (activeFilter.value === 'same') return props.unchanged
	return allIncoming.value
})

const incoming = computed<AddonRow[]>(() => ofType(incomingByChange.value, incomingType.value))

/**
 * Each axis counts what the *other* axis currently allows, so a pill always
 * reports what pressing it would leave. Which pills exist is decided by the
 * whole set, though — a category must not disappear from the row because the
 * change filter happens to exclude it, least of all the one selected.
 */
const incomingTypeFilters = computed(() => [
	{ value: 'all' as TypeKey, label: 'All types', count: incomingByChange.value.length },
	...typesIn(allIncoming.value).map((category) => ({
		value: category as TypeKey,
		label: categoryLabel(category),
		count: countOf(incomingByChange.value, category)
	}))
])

const changeFilters = computed(() => [
	{ value: 'all' as ChangeKey, label: 'All', count: countOf(allIncoming.value, incomingType.value) },
	{ value: 'new' as ChangeKey, label: 'New', count: countOf(props.added, incomingType.value) },
	{ value: 'updated' as ChangeKey, label: 'Updated', count: countOf(props.updated, incomingType.value) },
	{ value: 'same' as ChangeKey, label: 'Unchanged', count: countOf(props.unchanged, incomingType.value) }
])

const removedTypeFilters = computed(() => [
	{ value: 'all' as TypeKey, label: 'All types', count: props.removed.length },
	...typesIn(props.removed).map((category) => ({
		value: category as TypeKey,
		label: categoryLabel(category),
		count: countOf(props.removed, category)
	}))
])

const visibleRemoved = computed<AddonRow[]>(() => ofType(props.removed, removedType.value))

/**
 * "12 mods, 3 resource packs and 2 data packs".
 *
 * Withheld unless every deleted row is accounted for: a breakdown that does not
 * add up to the headline is a worse answer than no breakdown, on the one panel
 * where the numbers have to be trusted.
 */
const deletionBreakdown = computed(() =>
{
	const categories = typesIn(props.removed)
	const counted = categories.reduce((sum, category) => sum + countOf(props.removed, category), 0)
	if (counted !== props.removed.length) return ''
	return joinPhrases(categories.map((category) => categoryCountPhrase(category, countOf(props.removed, category))))
})

/**
 * A fetched update replaces the rows under a filter that was chosen for the
 * previous one. A change filter still means something against any diff, but a
 * category that is not in the new one leaves the list empty for a reason the
 * player never chose, so those selections go back to All.
 */
watch(incomingTypeFilters, (options) =>
{
	if (!options.some((option) => option.value === incomingType.value)) incomingType.value = 'all'
})

watch(removedTypeFilters, (options) =>
{
	if (!options.some((option) => option.value === removedType.value)) removedType.value = 'all'
})
</script>
