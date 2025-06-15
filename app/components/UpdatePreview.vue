<template>
  <div class="modal modal-open">
    <div class="modal-box w-11/12 max-w-5xl">
      <div class="flex justify-between items-center mb-4">
        <h2 class="text-2xl font-bold">
          Update Preview
        </h2>
        <button
          class="btn btn-sm btn-circle btn-ghost"
          @click="$emit('close')"
        >
          ✕
        </button>
      </div>

      <div
        v-if="!preview.hasChanges"
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
              {{ preview.diff.updated_addons.length }}
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
        </div>        <!-- Detailed Changes -->
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
            Updated ({{ preview.diff.updated_addons.length }})
          </button>
          <button
            class="tab"
            :class="{ 'tab-active': activeTab === 'removed' }"
            @click="activeTab = 'removed'"
          >
            Removed ({{ preview.diff.removed_addons.length }})
          </button>          <button
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
          <!-- New Addons -->
          <div
            v-if="activeTab === 'new'"
            class="space-y-2"
          >
            <div
              v-if="preview.diff.new_addons.length === 0"
              class="text-center text-gray-500"
            >
              No new addons to install
            </div>
            <div
              v-for="addonName in preview.diff.new_addons"
              :key="addonName"
              class="flex items-center justify-between p-3 bg-success/10 border border-success/20 rounded"
            >
              <div class="flex items-center space-x-2">
                <span class="w-4 h-4 bg-success rounded-full flex items-center justify-center">
                  <span class="text-xs text-white">+</span>
                </span>
                <span>{{ addonName }}</span>
              </div>
              <span class="badge badge-success">NEW</span>
            </div>
          </div>

          <!-- Updated Addons -->
          <div
            v-if="activeTab === 'updated'"
            class="space-y-2"
          >
            <div
              v-if="preview.diff.updated_addons.length === 0"
              class="text-center text-gray-500"
            >
              No addons to update
            </div>
            <div
              v-for="([oldVersion, newVersion], index) in preview.diff.updated_addons"
              :key="index"
              class="flex items-center justify-between p-3 bg-warning/10 border border-warning/20 rounded"
            >
              <div class="flex items-center space-x-2">
                <span class="w-4 h-4 bg-warning rounded-full flex items-center justify-center">
                  <span class="text-xs text-white">↑</span>
                </span>
                <span>Version {{ oldVersion }} → {{ newVersion }}</span>
              </div>
              <span class="badge badge-warning">UPDATE</span>
            </div>
          </div>

          <!-- Removed Addons -->
          <div
            v-if="activeTab === 'removed'"
            class="space-y-2"
          >
            <div
              v-if="preview.diff.removed_addons.length === 0"
              class="text-center text-gray-500"
            >
              No addons to remove
            </div>
            <div
              v-for="addonName in preview.diff.removed_addons"
              :key="addonName"
              class="flex items-center justify-between p-3 bg-error/10 border border-error/20 rounded"
            >
              <div class="flex items-center space-x-2">
                <span class="w-4 h-4 bg-error rounded-full flex items-center justify-center">
                  <span class="text-xs text-white">−</span>
                </span>
                <span>{{ addonName }}</span>
              </div>              <span class="badge badge-error">REMOVE</span>
            </div>
          </div>          <!-- Config Files -->
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
                  <span class="text-xs text-white">📄</span>
                </span>
                <div class="flex flex-col">
                  <span class="font-mono text-sm">{{ configFile.relative_path }}</span>
                  <span class="text-xs opacity-60">
                    {{ Math.round(configFile.content.length / 1024 * 100) / 100 }} KB
                  </span>
                </div>
              </div>
              <div class="flex items-center space-x-2">
                <span class="badge badge-info">CONFIG</span>
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
            This update will remove {{ preview.diff.removed_addons.length }} addon(s) and update {{ preview.diff.updated_addons.length }} addon(s).
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
	props.preview.diff.removed_addons.length > 0 || props.preview.diff.updated_addons.length > 0
)

// Set initial tab to the one with content
watch(
	() => props.preview,
	(newPreview) =>
	{
		if (newPreview.diff.new_addons.length > 0)
		{
			activeTab.value = 'new'
		}
		else if (newPreview.diff.updated_addons.length > 0)
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
