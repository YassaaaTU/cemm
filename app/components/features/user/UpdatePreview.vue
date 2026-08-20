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
          <p class="mt-0.5 text-xs text-base-content/65">
            {{ deletionNote }}
          </p>
        </div>
      </div>

      <AddonTable
        id="preview-removed"
        :title="applied ? 'Deleted' : 'Being deleted'"
        :rows="removed"
        :max-height="200"
        noun="addons"
      />
    </div>

    <AddonTable
      v-if="incoming.length > 0"
      id="preview-incoming"
      :title="applied ? 'Installed' : 'Incoming'"
      :rows="incoming"
      :max-height="300"
      noun="addons"
    >
      <template #filters>
        <div
          class="flex flex-wrap gap-1.5"
          role="group"
          aria-label="Filter by change"
        >
          <button
            v-for="option in filters"
            :key="option.value"
            type="button"
            class="cursor-pointer rounded-full border px-2.5 py-0.5 text-xs font-medium transition-colors duration-150 ease-(--ease-standard)"
            :class="activeFilter === option.value
              ? 'border-primary bg-primary/15 text-primary'
              : 'border-base-300 bg-base-100 text-base-content/60 hover:text-base-content'"
            :aria-pressed="activeFilter === option.value"
            @click="activeFilter = option.value"
          >
            {{ option.label }}
            <span class="ml-1 font-mono tabular-nums opacity-60">{{ option.count }}</span>
          </button>
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

type FilterKey = 'all' | 'new' | 'updated' | 'same'

const activeFilter = ref<FilterKey>('all')

const tallies = computed(() => [
	{ label: 'Added', count: props.added.length, tone: 'text-success' },
	{ label: 'Updated', count: props.updated.length, tone: 'text-info' },
	{ label: 'Deleted', count: props.removed.length, tone: 'text-error' },
	{ label: 'Untouched', count: props.unchanged.length, tone: 'text-base-content/70' }
])

const filters = computed(() => [
	{ value: 'all' as const, label: 'All', count: props.added.length + props.updated.length + props.unchanged.length },
	{ value: 'new' as const, label: 'New', count: props.added.length },
	{ value: 'updated' as const, label: 'Updated', count: props.updated.length },
	{ value: 'same' as const, label: 'Unchanged', count: props.unchanged.length }
])

/**
 * Added, updated and unchanged share one list with a filter, rather than three
 * stacked tables. Deletions are deliberately NOT in here — they keep their own
 * panel above, where a filter can never hide them.
 */
const incoming = computed<AddonRow[]>(() =>
{
	if (activeFilter.value === 'new') return props.added
	if (activeFilter.value === 'updated') return props.updated
	if (activeFilter.value === 'same') return props.unchanged
	return [...props.added, ...props.updated, ...props.unchanged]
})
</script>
