<template>
  <div class="addon-grid">
    <div class="addon-grid__header">
      <div class="addon-grid__header-left">
        <h2 class="addon-grid__title">
          {{ title }}
          <span class="addon-grid__count">{{ displayedAddons.length }}{{ searchTerm.length > 0 ? ` of ${totalAddons}` : '' }}</span>
        </h2>
      </div>

      <div class="addon-grid__header-right">
        <div
          v-if="addons.length >= 10"
          class="addon-grid__search"
        >
          <Icon
            name="mdi:magnify"
            size="1rem"
            class="addon-grid__search-icon"
          />
          <input
            v-model="searchTerm"
            type="text"
            :placeholder="`Search ${category}...`"
            class="addon-grid__search-input"
            :aria-label="`Search ${category} addons`"
          />
        </div>

        <div class="addon-grid__view-toggle">
          <button
            class="addon-grid__view-btn"
            :class="{ 'addon-grid__view-btn--active': currentLayout === 'grid' }"
            aria-label="Grid view"
            :aria-pressed="currentLayout === 'grid'"
            @click="currentLayout = 'grid'"
          >
            <Icon
              name="mdi:view-grid"
              size="1.1rem"
            />
          </button>
          <button
            class="addon-grid__view-btn"
            :class="{ 'addon-grid__view-btn--active': currentLayout === 'list' }"
            aria-label="List view"
            :aria-pressed="currentLayout === 'list'"
            @click="currentLayout = 'list'"
          >
            <Icon
              name="mdi:view-list"
              size="1.1rem"
            />
          </button>
        </div>
      </div>
    </div>

    <Transition name="bulk-bar">
      <div
        v-if="selectedAddonsInternal.length > 0"
        class="addon-grid__bulk"
      >
        <span class="addon-grid__bulk-count">{{ selectedAddonsInternal.length }} selected</span>
        <button
          v-if="showExclusion"
          class="btn btn-error btn-sm"
          @click="excludeSelected"
        >
          <Icon
            name="mdi:minus-circle-outline"
            size="1.1rem"
          />
          Exclude Selected
        </button>
        <button
          class="btn btn-ghost btn-sm"
          @click="clearSelection"
        >
          Clear Selection
        </button>
      </div>
    </Transition>

    <div
      v-if="isLoading"
      class="addon-grid__empty"
    >
      <LoadingSpinner
        :loading="true"
        size="sm"
      />
      <span class="addon-grid__empty-text">Loading addons...</span>
    </div>

    <div
      v-else-if="displayedAddons.length === 0"
      class="addon-grid__empty"
    >
      <Icon
        name="mdi:package-variant-closed"
        size="2.5rem"
        class="addon-grid__empty-icon"
      />
      <p class="addon-grid__empty-title">
        {{ searchTerm.length > 0 ? 'No addons match your search' : `No ${category} found` }}
      </p>
      <p
        v-if="searchTerm.length > 0"
        class="addon-grid__empty-hint"
      >
        Try a different search term
      </p>
    </div>

    <v-list
      v-else-if="useVirtualScrolling"
      v-slot="{ item: addon }"
      :data="displayedAddons"
      :style="{ height: `${containerHeight}px` }"
    >
      <AddonItem
        :key="`${addon.addon_project_id}-${addon.version}`"
        :addon="addon"
        :selected="selectedAddonsInternal.includes(addon.addon_name)"
        :show-selection="showSelection"
        :status="getAddonStatus(addon)"
        :excluded="excludedAddons.includes(addon.addon_name)"
        :show-exclusion="showExclusion"
        :layout="currentLayout"
        @toggle-selection="handleToggleSelection"
        @open-link="openCurseforge(addon)"
        @toggle-exclusion="handleToggleExclusion"
      />
    </v-list>

    <div
      v-else
      class="addon-grid__items"
      :class="`addon-grid__items--${currentLayout}`"
    >
      <AddonItem
        v-for="addon in displayedAddons"
        :key="`${addon.addon_project_id}-${addon.version}`"
        :addon="addon"
        :selected="selectedAddonsInternal.includes(addon.addon_name)"
        :show-selection="showSelection"
        :status="getAddonStatus(addon)"
        :excluded="excludedAddons.includes(addon.addon_name)"
        :show-exclusion="showExclusion"
        :layout="currentLayout"
        @toggle-selection="handleToggleSelection"
        @open-link="openCurseforge(addon)"
        @toggle-exclusion="handleToggleExclusion"
      />
    </div>

    <div
      v-if="showStats && isDev"
      class="addon-grid__stats"
    >
      <div>Total: {{ totalAddons }} | Visible: {{ visibleCount }} | Render ratio: {{ renderRatio }}%</div>
      <div>Search time: {{ searchDuration }}ms | Memory: ~{{ memoryEstimate }}KB</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { VList } from 'virtua/vue'

import type { Addon, ManifestUpdateInfo } from '~/types'

const { $logger: logger } = useNuxtApp()

interface Props
{
	addons: Addon[]
	title?: string
	category?: string
	selectedAddons?: string[]
	showSelection?: boolean
	isLoading?: boolean
	virtualScrollThreshold?: number
	containerHeight?: number
	showStats?: boolean
	updateInfo?: ManifestUpdateInfo | null
	excludedAddons?: string[]
	showExclusion?: boolean
	layout?: 'grid' | 'list'
}

const props = withDefaults(defineProps<Props>(), {
	title: 'Addons',
	category: 'addons',
	selectedAddons: () => [],
	showSelection: false,
	isLoading: false,
	virtualScrollThreshold: 100,
	containerHeight: 400,
	showStats: false,
	updateInfo: null,
	excludedAddons: () => [],
	showExclusion: false,
	layout: 'grid'
})

const emit = defineEmits<{
	toggleSelection: [addonName: string]
	toggleExclusion: [addonName: string]
	bulkActions: [action: 'exclude', addonNames: string[]]
}>()

const currentLayout = ref<'grid' | 'list'>(props.layout)
const selectedAddonsInternal = ref<string[]>([...props.selectedAddons])
const searchTerm = ref('')
const isSearching = computed(() => searchTerm.value.length > 0)

watch(() => props.selectedAddons, (val) =>
{
	selectedAddonsInternal.value = [...val]
}, { deep: true })

const filteredItems = computed(() =>
{
	if (searchTerm.value.length === 0) return props.addons

	const term = searchTerm.value.toLowerCase()
	return props.addons.filter((addon) =>
		addon.addon_name.toLowerCase().includes(term)
		|| addon.version.toLowerCase().includes(term)
	)
})

const useVirtualScrolling = computed(() => filteredItems.value.length > props.virtualScrollThreshold)

const displayedAddons = computed(() =>
	filteredItems.value.filter((a) => a.disabled !== true)
)
const totalAddons = computed(() => props.addons.length)
const visibleCount = computed(() => displayedAddons.value.length)
const renderRatio = computed(() => Math.round((visibleCount.value / Math.max(totalAddons.value, 1)) * 100))

const searchDuration = ref(0)
const memoryEstimate = computed(() => Math.round(visibleCount.value * 0.5))
const isDev = computed(() => import.meta.dev)

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

function handleToggleSelection(addonName: string)
{
	const idx = selectedAddonsInternal.value.indexOf(addonName)
	if (idx >= 0)
	{
		selectedAddonsInternal.value.splice(idx, 1)
	}
	else
	{
		selectedAddonsInternal.value.push(addonName)
	}
	emit('toggleSelection', addonName)
}

function handleToggleExclusion(addonName: string)
{
	emit('toggleExclusion', addonName)
}

function excludeSelected()
{
	emit('bulkActions', 'exclude', [...selectedAddonsInternal.value])
	clearSelection()
}

function clearSelection()
{
	selectedAddonsInternal.value = []
}

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

function getAddonStatus(addon: Addon): '' | 'added' | 'removed' | 'updated'
{
	if (props.updateInfo == null) return ''
	const added = props.updateInfo.addedAddons.find(
		(a: Addon) => a.addon_name === addon.addon_name && a.version === addon.version
	)
	const removed = props.updateInfo.removedAddons.includes(addon.addon_name)
	if (added != null) return 'added'
	if (removed) return 'removed'

	if (props.updateInfo.updated_addon_ids != null)
	{
		const updated = props.updateInfo.updated_addon_ids.includes(addon.addon_project_id)
		if (updated === true) return 'updated'
	}

	return ''
}

function getSelectedItems(): string[]
{
	return [...selectedAddonsInternal.value]
}

const getPerformanceStats = () => ({
	searchDuration: searchDuration.value,
	memoryEstimate: memoryEstimate.value,
	useVirtualScrolling: useVirtualScrolling.value
})

defineExpose({
	getPerformanceStats,
	searchTerm,
	filteredItems,
	getSelectedItems
})
</script>

<style scoped>
.addon-grid {
  width: 100%;
}

.addon-grid__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
  padding-bottom: var(--space-3);
  border-bottom: 1px solid var(--color-border-divider);
  margin-bottom: var(--space-4);
  flex-wrap: wrap;
}

.addon-grid__header-left {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.addon-grid__title {
  font-size: var(--text-heading-sm-size);
  font-weight: 600;
  color: var(--color-text-primary);
  margin: 0;
}

.addon-grid__count {
  display: inline-flex;
  align-items: center;
  padding: 1px 8px;
  border-radius: var(--radius-full);
  background: var(--color-surface-overlay);
  color: var(--color-text-secondary);
  font-size: var(--text-body-xs-size);
  font-weight: 600;
  margin-left: var(--space-2);
}

.addon-grid__header-right {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}

.addon-grid__search {
  position: relative;
  display: flex;
  align-items: center;
}

.addon-grid__search-icon {
  position: absolute;
  left: var(--space-2);
  color: var(--color-text-tertiary);
  pointer-events: none;
}

.addon-grid__search-input {
  height: 32px;
  padding: 0 var(--space-3) 0 var(--space-7);
  background: var(--color-surface-overlay);
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-md);
  color: var(--color-text-primary);
  font-size: var(--text-body-sm-size);
  width: 200px;
  transition: all var(--duration-fast);
}
.addon-grid__search-input:focus {
  outline: none;
  border-color: var(--color-accent-primary);
  box-shadow: 0 0 0 3px var(--color-accent-primary-muted);
}
.addon-grid__search-input::placeholder {
  color: var(--color-text-tertiary);
}

.addon-grid__view-toggle {
  display: flex;
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-md);
  overflow: hidden;
}

.addon-grid__view-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border: none;
  background: transparent;
  color: var(--color-text-tertiary);
  cursor: pointer;
  transition: all var(--duration-instant);
}
.addon-grid__view-btn--active {
  background: var(--color-accent-primary-muted);
  color: var(--color-accent-primary);
}
.addon-grid__view-btn:hover:not(.addon-grid__view-btn--active) {
  background: var(--color-surface-hover);
  color: var(--color-text-primary);
}
.addon-grid__view-btn:focus-visible {
  box-shadow: var(--focus-ring-offset);
  outline: none;
}

.addon-grid__bulk {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-3);
  background: var(--color-accent-primary-muted);
  border-radius: var(--radius-md);
  margin-bottom: var(--space-3);
}
.addon-grid__bulk-count {
  font-size: var(--text-body-sm-size);
  font-weight: 600;
  color: var(--color-accent-primary);
}

.addon-grid__items--grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: var(--space-3);
}
.addon-grid__items--list {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.addon-grid__empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--space-8);
  text-align: center;
}
.addon-grid__empty-icon {
  color: var(--color-text-tertiary);
  margin-bottom: var(--space-2);
}
.addon-grid__empty-title {
  font-size: var(--text-body-md-size);
  color: var(--color-text-secondary);
  margin: 0;
}
.addon-grid__empty-hint {
  font-size: var(--text-body-sm-size);
  color: var(--color-text-tertiary);
  margin: var(--space-1) 0 0;
}
.addon-grid__empty-text {
  font-size: var(--text-body-sm-size);
  color: var(--color-text-secondary);
  margin-left: var(--space-2);
}

.addon-grid__stats {
  margin-top: var(--space-4);
  padding: var(--space-3);
  background: var(--color-surface-raised);
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-md);
  font-family: var(--font-mono);
  font-size: var(--text-body-xs-size);
  color: var(--color-text-tertiary);
}

.bulk-bar-enter-active { transition: all var(--duration-normal) var(--ease-out); }
.bulk-bar-leave-active { transition: all var(--duration-fast) var(--ease-in); }
.bulk-bar-enter-from, .bulk-bar-leave-to { opacity: 0; transform: translateY(-8px); height: 0; padding: 0; margin: 0; overflow: hidden; }
</style>
