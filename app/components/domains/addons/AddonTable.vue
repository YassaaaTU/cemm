<template>
  <div
    class="overflow-hidden rounded-box border border-base-300 bg-base-200"
    :class="fill ? 'flex min-h-0 flex-1 flex-col' : ''"
  >
    <!-- List header: title, live count, category pills, search. -->
    <div class="flex shrink-0 flex-wrap items-center gap-x-3 gap-y-2 border-b border-base-300 px-4 py-3">
      <h3
        :id="`${id}-title`"
        class="text-[0.9375rem] font-semibold"
      >
        {{ title }}
      </h3>
      <span class="font-mono text-[0.8125rem] text-base-content/50 tabular-nums">
        {{ visibleRows.length }}<template v-if="visibleRows.length !== rows.length"> / {{ rows.length }}</template>
      </span>

      <slot name="filters" />

      <div class="flex-1" />

      <slot name="actions" />

      <label
        v-if="rows.length > 8"
        class="input w-full max-w-68 border-base-300 bg-base-100 input-md"
      >
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
          :placeholder="`Search ${rows.length} ${noun}…`"
          :aria-label="`Search ${title}`"
        />
      </label>
    </div>

    <!-- Column heads -->
    <div
      class="grid shrink-0 items-center gap-4 border-b border-base-300/60 px-4 py-2 text-xs font-medium text-base-content/60"
      :class="gridClass"
      role="presentation"
    >
      <span v-if="variant === 'review'">Status</span>
      <span v-else />
      <span>{{ variant === 'review' ? 'Addon' : 'Project' }}</span>
      <span>Version</span>
      <span
        v-if="showActions"
        class="text-right"
      >{{ actionsLabel }}</span>
    </div>

    <p
      v-if="visibleRows.length === 0"
      class="px-4 py-10 text-center text-sm text-base-content/50"
    >
      <template v-if="search.length > 0">
        Nothing matches “{{ search }}”.
      </template>
      <template v-else>
        {{ emptyLabel }}
      </template>
    </p>

    <!--
      One rendering path at every list length. These run from a handful of
      datapacks to several hundred mods, and virtualising only above a threshold
      meant short and long lists took different code paths with different
      semantics. ARIA grid roles carry the table semantics.
    -->
    <div
      v-else
      ref="listWrapper"
      role="table"
      :aria-labelledby="`${id}-title`"
      :aria-rowcount="visibleRows.length"
      :class="fill ? 'min-h-0 flex-1' : ''"
    >
      <v-list
        v-slot="{ item: row }"
        :data="visibleRows"
        :style="listStyle"
      >
        <div
          :key="row.key"
          role="row"
          class="grid items-center gap-4 border-b border-base-300/40 px-4 text-sm transition-[background-color,opacity] duration-150 ease-(--ease-standard) last:border-b-0 hover:bg-base-300/40"
          :class="[gridClass, row.dimmed === true ? 'bg-error/6' : '']"
        >
          <span
            v-if="variant === 'review'"
            role="cell"
          >
            <StatusChip
              :tone="row.tone"
              :label="row.label"
            />
          </span>
          <span
            v-else
            role="cell"
          />

          <span
            role="cell"
            class="flex min-w-0 items-center gap-3 py-2"
          >
            <AddonThumb
              :name="row.name"
              :src="row.thumbnailUrl ?? ''"
            />
            <span class="min-w-0">
              <span
                class="block truncate text-[0.9375rem] font-medium"
                :class="row.struck === true ? 'text-base-content/60 line-through decoration-error' : ''"
                :title="row.name"
              >{{ row.name }}</span>
              <span
                v-if="row.subtitle.length > 0"
                class="block truncate font-mono text-xs text-base-content/60"
                :title="row.subtitle"
              >{{ row.subtitle }}</span>
            </span>
          </span>

          <span
            role="cell"
            class="min-w-0"
            :class="row.dimmed === true ? 'opacity-50' : ''"
          >
            <span
              class="block truncate font-mono text-sm font-semibold tabular-nums"
              :title="row.version"
            >
              {{ row.version.length > 0 ? row.version : '—' }}
            </span>
            <!-- Only rendered when it says something the name column does not.
                 Setting this to the filename duplicated it in two columns. -->
            <span
              v-if="row.versionNote.length > 0"
              class="block truncate text-xs text-base-content/60"
              :title="row.versionNote"
            >{{ row.versionNote }}</span>
          </span>

          <span
            v-if="showActions"
            role="cell"
            class="flex items-center justify-end gap-1"
          >
            <slot
              name="row-action"
              :row="row"
            />
          </span>
        </div>
      </v-list>
    </div>
  </div>
</template>

<script setup lang="ts">
import { VList } from 'virtua/vue'

import type { StatusTone } from '~/components/shared/ui/StatusChip.vue'

export interface AddonRow
{
	key: string
	name: string
	/** Second line under the name — the on-disk filename, or why it is excluded. */
	subtitle: string
	version: string
	/** Second line under the version. Leave empty unless it adds information the
	 *  name column does not already carry. */
	versionNote: string
	tone: StatusTone
	label: string
	/** Struck through: being removed or excluded. */
	struck?: boolean
	/** Whole row de-emphasised. */
	dimmed?: boolean
	thumbnailUrl?: string
}

const props = withDefaults(
	defineProps<{
		id: string
		title: string
		rows: AddonRow[]
		variant?: 'review' | 'manage'
		emptyLabel?: string
		showActions?: boolean
		actionsLabel?: string
		noun?: string
		/** Cap the list height. Ignored when `fill` is set. */
		maxHeight?: number
		/** Grow to fill the available height instead of capping. Used where the
		 *  list IS the screen, so a 500-row list does not sit in a letterbox
		 *  with dead space beneath it. */
		fill?: boolean
	}>(),
	{
		variant: 'review',
		emptyLabel: 'Nothing here yet.',
		showActions: false,
		actionsLabel: 'Actions',
		noun: 'addons',
		maxHeight: 340,
		fill: false
	}
)

const search = ref('')

/**
 * The admin panel swaps categories through the same component instance rather
 * than remounting it, so a term typed while browsing Mods used to survive into
 * Data packs — where it hid a list that was not actually empty behind
 * "Nothing matches". The id changes with the dataset, so it is the signal that
 * this is a different list and the filter no longer applies.
 */
watch(() => props.id, () =>
{
	search.value = ''
})

const gridClass = computed(() =>
{
	if (props.variant === 'review')
	{
		return props.showActions
			? 'grid-cols-[5.5rem_minmax(0,1fr)_10rem_5rem]'
			: 'grid-cols-[5.5rem_minmax(0,1fr)_10rem]'
	}
	return props.showActions
		? 'grid-cols-[0_minmax(0,1fr)_11rem_5.5rem]'
		: 'grid-cols-[0_minmax(0,1fr)_11rem]'
})

const visibleRows = computed(() =>
{
	const term = search.value.trim().toLowerCase()
	if (term.length === 0) return props.rows
	return props.rows.filter(
		(row) =>
			row.name.toLowerCase().includes(term)
			|| row.subtitle.toLowerCase().includes(term)
			|| row.version.toLowerCase().includes(term)
	)
})

const ROW_HEIGHT = 58

const listWrapper = ref<HTMLElement | null>(null)
const measuredHeight = ref(0)
let observer: ResizeObserver | null = null

/**
 * In fill mode the height is measured rather than set to `height: 100%`.
 * virtua reads its viewport size once on mount, and a percentage height against
 * a flex parent still measures 0 at that moment — which rendered an empty list.
 * Observing the wrapper gives it a real pixel height and keeps it correct when
 * the window resizes.
 */
onMounted(() =>
{
	if (!props.fill || listWrapper.value === null) return

	observer = new ResizeObserver((entries) =>
	{
		const height = entries[0]?.contentRect.height ?? 0
		if (height > 0) measuredHeight.value = height
	})
	observer.observe(listWrapper.value)
	measuredHeight.value = listWrapper.value.getBoundingClientRect().height
})

onUnmounted(() =>
{
	observer?.disconnect()
	observer = null
})

/**
 * Filling takes the measured height; otherwise the list grows with its contents
 * up to a ceiling, so a four-row datapack list does not reserve the same slab
 * of space as a 300-row mod list.
 */
const listStyle = computed(() =>
{
	if (props.fill)
	{
		// Falls back to the cap until the first measurement lands.
		return { height: `${measuredHeight.value > 0 ? measuredHeight.value : props.maxHeight}px` }
	}
	return {
		height: `${Math.min(Math.max(visibleRows.value.length, 1) * ROW_HEIGHT, props.maxHeight)}px`
	}
})
</script>
