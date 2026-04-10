<template>
  <div class="modal modal-open">
    <div class="modal-box w-11/12 max-w-5xl max-h-[calc(100vh-10rem)]">
      <div class="flex justify-between items-center mb-4">
        <div>
          <h2 class="text-2xl font-bold">
            Update Preview
          </h2>
          <div class="text-sm opacity-70 mt-1">
            <span
              v-if="preview.newManifest.updateType === 'config'"
              class="badge badge-info"
            >
              Config-Only Update
            </span>
            <span
              v-else
              class="badge badge-primary"
            >
              Full Update
            </span>
          </div>
        </div>
        <button
          class="btn btn-sm btn-circle btn-ghost"
          @click="$emit('close')"
        >
          ✕
        </button>
      </div>

      <div
        v-if="!preview.hasChanges && preview.newManifest.updateType === 'config'"
        class="alert alert-info mb-4"
      >
        <span>Config-only update: Only configuration files will be updated. No addons will be modified.</span>
      </div>

      <div
        v-else-if="!preview.hasChanges"
        class="alert alert-info mb-4"
      >
        <span>No changes detected. This appears to be a fresh installation.</span>
      </div>

      <div
        v-else
        class="space-y-6"
      >
        <!-- Summary -->
        <div class="stats shadow w-full">
          <div class="stat">
            <div class="stat-title">
              New Addons
            </div>
            <div class="stat-value text-success">
              {{ preview.diff.new_addons.length }}
            </div>
          </div>
          <div class="stat">
            <div class="stat-title">
              Updated Addons
            </div>
            <div class="stat-value text-warning">
              {{ preview.diff.updated_addon_ids.length }}
            </div>
          </div>
          <div class="stat">
            <div class="stat-title">
              Removed Addons
            </div>
            <div class="stat-value text-error">
              {{ preview.diff.removed_addons.length }}
            </div>
          </div>
        </div>

        <!-- Detailed Changes -->
        <div class="tabs tabs-boxed">
          <button
            class="tab"
            :class="{ 'tab-active': activeTab === 'new' }"
            @click="activeTab = 'new'"
          >
            New ({{ preview.diff.new_addons.length }})
          </button>
          <button
            class="tab"
            :class="{ 'tab-active': activeTab === 'updated' }"
            @click="activeTab = 'updated'"
          >
            Updated ({{ preview.diff.updated_addon_ids.length }})
          </button>
          <button
            class="tab"
            :class="{ 'tab-active': activeTab === 'removed' }"
            @click="activeTab = 'removed'"
          >
            Removed ({{ preview.diff.removed_addons.length }})
          </button>
          <button
            v-if="preview.configFiles && preview.configFiles.length > 0"
            class="tab"
            :class="{ 'tab-active': activeTab === 'config' }"
            @click="activeTab = 'config'"
          >
            Config Files ({{ preview.configFiles.length }})
            <span
              v-if="!configFilesDownloaded"
              class="badge badge-warning badge-xs ml-1"
            >
              Not Downloaded
            </span>
          </button>
        </div>

        <!-- Tab Content -->
        <div class="min-h-48">
          <!-- New Addons by Category -->
          <div
            v-if="activeTab === 'new'"
            class="space-y-4"
          >
            <div
              v-if="preview.diff.new_addons.length === 0"
              class="text-center text-gray-500"
            >
              No new addons to install
            </div>
            <template v-else>
              <AddonCategorySection
                v-for="category in visibleCategories.new"
                :key="category.key"
                :title="category.title"
                :icon="category.icon"
                :items="categorizedNewAddons[category.key]"
                type="new"
              />
            </template>
          </div>

          <!-- Updated Addons by Category -->
          <div
            v-if="activeTab === 'updated'"
            class="space-y-4"
          >
            <div
              v-if="preview.diff.updated_addon_ids.length === 0"
              class="text-center text-gray-500"
            >
              No addons to update
            </div>
            <template v-else>
              <AddonCategorySection
                v-for="category in visibleCategories.updated"
                :key="category.key"
                :title="category.title"
                :icon="category.icon"
                :items="categorizedUpdatedAddons[category.key]"
                type="updated"
              />
            </template>
          </div>

          <!-- Removed Addons by Category -->
          <div
            v-if="activeTab === 'removed'"
            class="space-y-4"
          >
            <div
              v-if="preview.diff.removed_addons.length === 0"
              class="text-center text-gray-500"
            >
              No addons to remove
            </div>
            <template v-else>
              <AddonCategorySection
                v-for="category in visibleCategories.removed"
                :key="category.key"
                :title="category.title"
                :icon="category.icon"
                :items="categorizedRemovedAddons[category.key]"
                type="removed"
              />
            </template>
          </div>

          <!-- Config Files -->
          <div
            v-if="activeTab === 'config'"
            class="space-y-4"
          >
            <div
              v-if="!configFilesDownloaded && preview.configFiles && preview.configFiles.length > 0"
              class="alert alert-info"
            >
              <svg
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
                class="w-6 h-6 stroke-current shrink-0"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
              <span>Config files will be downloaded when you confirm the update.</span>
            </div>

            <div
              v-if="!preview.configFiles || preview.configFiles.length === 0"
              class="text-center text-gray-500"
            >
              No config files to install
            </div>
            <div
              v-for="configFile in preview.configFiles"
              :key="configFile.relative_path"
              class="flex items-center justify-between p-3 bg-info/10 border border-info/20 rounded"
            >
              <div class="flex items-center space-x-2">
                <span class="w-4 h-4 bg-info rounded-full flex items-center justify-center">
                  <span class="text-xs text-white">
                    {{ configFile.is_binary ? '📦' : '📄' }}
                  </span>
                </span>
                <div class="flex flex-col">
                  <span class="font-mono text-sm">{{ configFile.relative_path }}</span>
                  <span class="text-xs opacity-60">
                    <template v-if="configFile.is_binary">
                      Binary file ({{ configFile.filename.split('.').pop()?.toUpperCase() || 'BINARY' }})
                    </template>
                    <template v-else>
                      {{ Math.round(configFile.content.length / 1024 * 100) / 100 }} KB
                    </template>
                  </span>
                </div>
              </div>
              <div class="flex items-center space-x-2">
                <span
                  class="badge"
                  :class="configFile.is_binary ? 'badge-secondary' : 'badge-info'"
                >
                  {{ configFile.is_binary ? 'BINARY' : 'CONFIG' }}
                </span>
                <span
                  v-if="!configFilesDownloaded"
                  class="badge badge-warning badge-xs"
                >
                  Pending
                </span>
              </div>
            </div>
          </div>
        </div>

        <!-- Warning for Destructive Changes -->
        <div
          v-if="hasDestructiveChanges"
          class="alert alert-warning"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            class="stroke-current shrink-0 h-6 w-6"
            fill="none"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z"
            />
          </svg>
          <span>
            This update will remove {{ preview.diff.removed_addons.length }} addon(s) and update {{ preview.diff.updated_addon_ids.length }} addon(s).
            Old files will be deleted. This action cannot be undone.
          </span>
        </div>
      </div>

      <!-- Action Buttons -->
      <div class="modal-action">
        <button
          class="btn btn-ghost"
          @click="$emit('close')"
        >
          Cancel
        </button>
        <button
          class="btn btn-primary"
          :disabled="installing"
          @click="$emit('confirm')"
        >
          <span v-if="!installing">
            {{ preview.hasChanges ? 'Apply Update' : 'Install' }}
          </span>
          <span
            v-else
            class="loading loading-spinner"
          />
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { UpdatePreview } from '~/types'

interface Props
{
	preview: UpdatePreview
	installing?: boolean
	configFilesDownloaded?: boolean
}

interface Emits
{
	close: []
	confirm: []
}

const props = withDefaults(defineProps<Props>(), {
	installing: false,
	configFilesDownloaded: false
})

defineEmits<Emits>()

const activeTab = ref<'new' | 'updated' | 'removed' | 'config'>('new')

const hasDestructiveChanges = computed(() =>
	props.preview.diff.removed_addons.length > 0 || props.preview.diff.updated_addon_ids.length > 0
)

// Category definitions with icons and titles
const categoryDefinitions = [
	{ key: 'mods' as const, title: 'Mods', icon: '🎮' },
	{ key: 'resourcepacks' as const, title: 'Resource Packs', icon: '🎨' },
	{ key: 'shaderpacks' as const, title: 'Shader Packs', icon: '✨' },
	{ key: 'datapacks' as const, title: 'Data Packs', icon: '📦' }
]

// Helper to get addon info by project ID (includes category)
const getAddonInfoByProjectId = (projectId: number): { name: string, category: string } =>
{
	const categories = [
		{ key: 'mods', addons: props.preview.newManifest.mods },
		{ key: 'resourcepacks', addons: props.preview.newManifest.resourcepacks },
		{ key: 'shaderpacks', addons: props.preview.newManifest.shaderpacks },
		{ key: 'datapacks', addons: props.preview.newManifest.datapacks }
	]

	for (const cat of categories)
	{
		const addon = cat.addons.find((a) => a.addon_project_id === projectId)
		if (addon !== undefined)
		{
			return { name: addon.addon_name, category: cat.key }
		}
	}

	// Check old manifest if not found in new
	const oldCategories = [
		{ key: 'mods', addons: props.preview.oldManifest?.mods ?? [] },
		{ key: 'resourcepacks', addons: props.preview.oldManifest?.resourcepacks ?? [] },
		{ key: 'shaderpacks', addons: props.preview.oldManifest?.shaderpacks ?? [] },
		{ key: 'datapacks', addons: props.preview.oldManifest?.datapacks ?? [] }
	]

	for (const cat of oldCategories)
	{
		const addon = cat.addons.find((a) => a.addon_project_id === projectId)
		if (addon !== undefined)
		{
			return { name: addon.addon_name, category: cat.key }
		}
	}

	return { name: `Unknown (ID: ${projectId})`, category: 'mods' }
}

// Categorize new addons by name
const categorizedNewAddons = computed(() =>
{
	const result = {
		mods: [] as string[],
		resourcepacks: [] as string[],
		shaderpacks: [] as string[],
		datapacks: [] as string[]
	}

	for (const addonName of props.preview.diff.new_addons)
	{
		// Find which category this addon belongs to
		const categories = [
			{ key: 'mods' as const, addons: props.preview.newManifest.mods },
			{ key: 'resourcepacks' as const, addons: props.preview.newManifest.resourcepacks },
			{ key: 'shaderpacks' as const, addons: props.preview.newManifest.shaderpacks },
			{ key: 'datapacks' as const, addons: props.preview.newManifest.datapacks }
		]

		for (const cat of categories)
		{
			if (cat.addons.some((a) => a.addon_name === addonName))
			{
				result[cat.key].push(addonName)
				break
			}
		}
	}

	return result
})

// Categorize updated addons by project ID
const categorizedUpdatedAddons = computed(() =>
{
	const result = {
		mods: [] as string[],
		resourcepacks: [] as string[],
		shaderpacks: [] as string[],
		datapacks: [] as string[]
	}

	for (const projectId of props.preview.diff.updated_addon_ids)
	{
		const info = getAddonInfoByProjectId(projectId)
		result[info.category as keyof typeof result].push(info.name)
	}

	return result
})

// Categorize removed addons by name
const categorizedRemovedAddons = computed(() =>
{
	const result = {
		mods: [] as string[],
		resourcepacks: [] as string[],
		shaderpacks: [] as string[],
		datapacks: [] as string[]
	}

	for (const addonName of props.preview.diff.removed_addons)
	{
		// Check old manifest first (since it was removed, it should be there)
		const oldCategories = [
			{ key: 'mods' as const, addons: props.preview.oldManifest?.mods ?? [] },
			{ key: 'resourcepacks' as const, addons: props.preview.oldManifest?.resourcepacks ?? [] },
			{ key: 'shaderpacks' as const, addons: props.preview.oldManifest?.shaderpacks ?? [] },
			{ key: 'datapacks' as const, addons: props.preview.oldManifest?.datapacks ?? [] }
		]

		let found = false
		for (const cat of oldCategories)
		{
			if (cat.addons.some((a) => a.addon_name === addonName))
			{
				result[cat.key].push(addonName)
				found = true
				break
			}
		}

		// Fallback to new manifest if not found in old
		if (!found)
		{
			const newCategories = [
				{ key: 'mods' as const, addons: props.preview.newManifest.mods },
				{ key: 'resourcepacks' as const, addons: props.preview.newManifest.resourcepacks },
				{ key: 'shaderpacks' as const, addons: props.preview.newManifest.shaderpacks },
				{ key: 'datapacks' as const, addons: props.preview.newManifest.datapacks }
			]

			for (const cat of newCategories)
			{
				if (cat.addons.some((a) => a.addon_name === addonName))
				{
					result[cat.key].push(addonName)
					break
				}
			}
		}
	}

	return result
})

// Compute which categories have items for each tab
const visibleCategories = computed(() => ({
	new: categoryDefinitions.filter((cat) => categorizedNewAddons.value[cat.key].length > 0),
	updated: categoryDefinitions.filter((cat) => categorizedUpdatedAddons.value[cat.key].length > 0),
	removed: categoryDefinitions.filter((cat) => categorizedRemovedAddons.value[cat.key].length > 0)
}))

// Set initial tab to the one with content
watch(
	() => props.preview,
	(newPreview) =>
	{
		if (newPreview.diff.new_addons.length > 0)
		{
			activeTab.value = 'new'
		}
		else if (newPreview.diff.updated_addon_ids.length > 0)
		{
			activeTab.value = 'updated'
		}
		else if (newPreview.diff.removed_addons.length > 0)
		{
			activeTab.value = 'removed'
		}
	},
	{ immediate: true }
)
</script>
