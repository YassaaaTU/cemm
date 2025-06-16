<template>
  <div>
    <!-- Header with search and stats -->
    <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 mb-4 border-b border-base-300 pb-4">
      <h2 class="font-semibold text-lg">
        {{ title }}
        <span class="badge badge-neutral badge-sm ml-2">
          {{ displayedAddons.length }}{{ searchTerm.length > 0 ? ` of ${totalAddons}` : '' }}
        </span>
      </h2>

      <!-- Search box for large lists -->
      <div
        v-if="addons.length >= searchThreshold"
        class="w-full sm:w-auto"
      >
        <input
          v-model="searchTerm"
          type="text"
          placeholder="Search addons..."
          class="input input-bordered input-sm w-full sm:w-64"
          :aria-label="`Search ${category} addons`"
        />
      </div>
    </div>

    <!-- Loading state -->
    <div
      v-if="isLoading"
      class="flex items-center justify-center p-8"
    >
      <LoadingSpinner
        :loading="isLoading"
        size="sm"
      />
      <span class="ml-2 text-sm opacity-70">Loading addons...</span>
    </div>

    <!-- Empty state -->
    <div
      v-else-if="displayedAddons.length === 0"
      class="text-center p-8 bg-base-200 rounded-lg"
    >
      <div class="text-base-content opacity-60">
        <Icon
          name="mdi:emoticon-sad-outline"
          size="2rem"
          class="mb-2"
        />
        <p class="text-sm">
          {{ searchTerm.length > 0 ? 'No addons match your search' : `No ${category} found` }}
        </p>
        <p
          v-if="searchTerm.length > 0"
          class="text-xs mt-1 opacity-50"
        >
          Try a different search term
        </p>
      </div>
    </div>

    <!-- Virtual scrolling container for large lists -->
    <v-list
      v-if="useVirtualScrolling"
      v-slot="{ item: addon, index }"
      :data="displayedAddons"
      :style="{ height: `${containerHeight}px` }"
    >
      <addon-item
        :key="`${addon.addon_project_id}-${addon.version}`"
        :addon="addon"
        :index="index"
        :selected="selectedAddons.includes(addon.addon_name)"
        :show-selection="showSelection"
        :status="getAddonStatus(addon)"
        @toggle-selection="handleToggleSelection"
        @open-link="openCurseforge(addon)"
      />
    </v-list>

    <!-- Regular rendering for smaller lists -->
    <div
      v-else
      class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4"
    >
      <addon-item
        v-for="(addon, index) in displayedAddons"
        :key="`${addon.addon_project_id}-${addon.version}`"
        :addon="addon"
        :index="index"
        :selected="selectedAddons.includes(addon.addon_name)"
        :show-selection="showSelection"
        :status="getAddonStatus(addon)"
        @toggle-selection="handleToggleSelection"
        @open-link="openCurseforge(addon)"
      />
    </div>
    <!-- Performance stats (dev only) -->
    <div
      v-if="showStats && isDev"
      class="mt-4 collapse bg-base-100 border-base-300 border"
    >
      <input
        type="checkbox"
        class="peer"
      />
      <div class="collapse-title bg-primary text-primary-content peer-checked:bg-secondary peer-checked:text-secondary-content">
        Performance Stats
      </div>
      <div class="collapse-content bg-primary text-primary-content peer-checked:bg-secondary peer-checked:text-secondary-content">
        <div>Total: {{ totalAddons }} | Visible: {{ visibleCount }} | Render ratio: {{ renderRatio }}%</div>
        <div>Search time: {{ searchDuration }}ms | Memory: ~{{ memoryEstimate }}KB</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { VList } from 'virtua/vue'

import { useSearchOptimized } from '~/composables/usePerformance'
import type { Addon, UpdateInfo } from '~/types'

const logger = usePinoLogger()

interface Props
{
	addons: Addon[]
	title?: string
	category?: string
	selectedAddons?: string[]
	showSelection?: boolean
	isLoading?: boolean
	searchThreshold?: number
	virtualScrollThreshold?: number
	containerHeight?: number
	showStats?: boolean
	updateInfo?: UpdateInfo | null
}

const props = withDefaults(defineProps<Props>(), {
	title: 'Addons',
	category: 'addons',
	selectedAddons: () => [],
	showSelection: false,
	isLoading: false,
	searchThreshold: 20,
	virtualScrollThreshold: 100,
	containerHeight: 400,
	showStats: false,
	updateInfo: null
})

// Composables
const { searchTerm, isSearching, filteredItems } = useSearchOptimized(
	computed(() => props.addons),
	['addon_name', 'version'],
	{
		debounceMs: 200,
		minLength: 1,
		maxResults: 500
	}
)

// Virtual scrolling for large lists
const useVirtualScrolling = computed(() => filteredItems.value.length > props.virtualScrollThreshold)

// Event handlers
const emit = defineEmits<{ toggleSelection: [addonName: string] }>()

const handleToggleSelection = (addonName: string) =>
{
	emit('toggleSelection', addonName)
}

// Open CurseForge/addon URL logic (from ManifestPreview)
async function openCurseforge(addon: Addon)
{
	logger.info('Opening CurseForge page for addon', {
		addonName: addon.addon_name,
		webSiteURL: addon.webSiteURL,
		hasWebSiteURL: !(addon.webSiteURL == null)
	})

	if ((addon.webSiteURL != null) && addon.webSiteURL.length > 0)
	{
		logger.info('Attempting to open URL:', addon.webSiteURL)
		try
		{
			await invoke('open_url', { url: addon.webSiteURL })
			logger.info('Successfully called open_url')
		}
		catch (e)
		{
			logger.error('Failed to open URL', { url: addon.webSiteURL, error: e })
		}
	}
	else
	{
		logger.warn('No valid webSiteURL found for addon', {
			addonName: addon.addon_name,
			webSiteURL: addon.webSiteURL,
			webSiteURLType: typeof addon.webSiteURL
		})
	}
}

// Computed properties
const displayedAddons = computed(() =>
	filteredItems.value.filter((a) => a.disabled !== true)
)
const totalAddons = computed(() => props.addons.length)
const visibleCount = computed(() => displayedAddons.value.length)
const renderRatio = computed(() => Math.round((visibleCount.value / Math.max(totalAddons.value, 1)) * 100))

// Performance metrics
const searchDuration = ref(0)
const memoryEstimate = computed(() => Math.round(visibleCount.value * 0.5)) // ~0.5KB per addon estimate
const isDev = computed(() => import.meta.dev)

// Watch search performance
watch(isSearching, (searching) =>
{
	if (searching)
	{
		const start = performance.now()
		const unwatch = watch(isSearching, (stillSearching) =>
		{
			if (!stillSearching)
			{
				searchDuration.value = Math.round(performance.now() - start)
				unwatch()
			}
		})
	}
})

// Addon status logic for highlighting (added/removed)
function getAddonStatus(addon: Addon): '' | 'added' | 'removed'
{
	if (props.updateInfo == null) return ''
	const added = props.updateInfo.addedAddons.find(
		(a: Addon) => a.addon_name === addon.addon_name && a.version === addon.version
	)
	const removed = props.updateInfo.removedAddons.includes(addon.addon_name)
	if (added != null) return 'added'
	if (removed) return 'removed'
	return ''
}

// Expose stats for debugging
const getPerformanceStats = () => ({
	searchDuration: searchDuration.value,
	memoryEstimate: memoryEstimate.value,
	useVirtualScrolling: useVirtualScrolling.value
})

defineExpose({
	getPerformanceStats,
	searchTerm,
	filteredItems
})
</script>
