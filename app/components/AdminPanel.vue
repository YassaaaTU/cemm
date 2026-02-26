<template>
  <div
    role="main"
    aria-labelledby="admin-mode-title"
  >
    <h2
      id="admin-mode-title"
      class="text-2xl font-bold mb-4"
    >
      Admin Mode
    </h2>

    <!-- Action Buttons Section -->
    <section
      aria-labelledby="admin-actions-section"
      class="mb-6"
    >
      <div class="flex flex-wrap gap-2">
        <button
          class="btn btn-primary"
          aria-describedby="load-instance-help"
          @click="handleLoadInstance"
        >
          Load Instance
        </button>

        <button
          class="btn btn-secondary"
          :disabled="manifest == null"
          :aria-describedby="manifest == null ? 'save-disabled-help' : 'save-manifest-help'"
          @click="handleSaveManifest"
        >
          Save Manifest
        </button>
        <button
          class="btn btn-accent"
          :disabled="(manifest == null && selectedConfigFiles.length === 0) || uploading"
          :aria-describedby="getUploadButtonDescription()"
          @click="handleUploadToGithub"
        >
          <span v-if="!uploading">
            {{ getUploadButtonText() }}
            <span
              v-if="selectedConfigFiles.length > 0"
              class="badge badge-secondary badge-sm ml-1"
              :aria-label="`${selectedConfigFiles.length} config files selected`"
            >
              +{{ selectedConfigFiles.length }} config
            </span>
          </span>
          <loading-spinner
            v-else
            :loading="true"
            size="sm"
            message="Uploading..."
            aria-label="Uploading manifest and config files to GitHub"
          />
        </button>
      </div>
    </section>

    <div class="mt-6 flex flex-col gap-2">
      <progress-bar
        :progress="progress"
        :label="uploading ? 'Uploading to GitHub...' : ''"
        :color="uploading ? 'primary' : 'success'"
        :show-percentage="uploading"
      />
    </div>

    <!-- Status messages -->
    <app-alert
      v-if="statusMessage"
      class="mt-4"
      :message="statusMessage"
      :type="statusType"
      @close="clearStatus"
    />

    <div
      v-if="manifest"
      class="mt-4"
    >
      <!-- Exclusion summary -->
      <div
        v-if="excludedCount > 0"
        class="mb-4 p-3 bg-warning/10 border border-warning/30 rounded-lg"
      >
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <Icon
              name="mdi:information-outline"
              class="text-warning"
            />
            <span class="text-sm">
              <strong>{{ excludedCount }}</strong> addon(s) excluded from upload
            </span>
          </div>
          <button
            class="btn btn-ghost btn-xs text-warning"
            @click="clearAllExclusions"
          >
            Clear All
          </button>
        </div>
      </div>

      <addon-list
        :addons="manifest.mods"
        :update-info="manifestStore.updateInfo"
        :excluded-addons="manifestStore.excludedAddons"
        :show-exclusion="true"
        title="Mods"
        category="mods"
        class="mb-4"
        @toggle-exclusion="handleToggleExclusion"
      />
      <addon-list
        v-if="manifest.resourcepacks.length > 0"
        :addons="manifest.resourcepacks"
        :update-info="manifestStore.updateInfo"
        :excluded-addons="manifestStore.excludedAddons"
        :show-exclusion="true"
        title="Resource Packs"
        category="resourcepacks"
        class="mb-4"
        @toggle-exclusion="handleToggleExclusion"
      />
      <addon-list
        v-if="manifest.shaderpacks.length > 0"
        :addons="manifest.shaderpacks"
        :update-info="manifestStore.updateInfo"
        :excluded-addons="manifestStore.excludedAddons"
        :show-exclusion="true"
        title="Shader Packs"
        category="shaderpacks"
        class="mb-4"
        @toggle-exclusion="handleToggleExclusion"
      />
      <addon-list
        v-if="manifest.datapacks.length > 0"
        :addons="manifest.datapacks"
        :update-info="manifestStore.updateInfo"
        :excluded-addons="manifestStore.excludedAddons"
        :show-exclusion="true"
        title="Data Packs"
        category="datapacks"
        @toggle-exclusion="handleToggleExclusion"
      />
    </div>

    <div
      v-else
      class="space-y-4"
    >
      <!-- Skeleton loading state -->
      <div class="flex items-center justify-center p-8 text-center">
        <div class="space-y-3">
          <div class="text-gray-400 text-sm">
            No manifest loaded
          </div>
          <div class="text-xs opacity-60">
            Load a minecraftinstance.json file to see addon details
          </div>
          <!-- Skeleton placeholder -->
          <div class="space-y-2 mt-4">
            <div class="skeleton h-4 w-32 mx-auto" />
            <div class="skeleton h-3 w-24 mx-auto" />
            <div class="skeleton h-3 w-28 mx-auto" />
          </div>
        </div>
      </div>
    </div>

    <!-- Config Files Section -->
    <config-files-section
      v-model="selectedConfigFiles"
      class="mt-6"
      @status="handleStatus"
    />
  </div>
</template>

<script setup lang="ts">
import type { ConfigFileWithContent } from '~/types'

const { loadInstance, saveManifest, uploadToGithub } = useAdminApi()
const manifestStore = useManifestStore()
const { $logger: logger } = useNuxtApp()

// Component state
const uploading = ref(false)
const progress = ref(0)
const statusMessage = ref('')
const statusType = ref<'success' | 'error' | 'info' | 'warning'>('info')
const selectedConfigFiles = ref<ConfigFileWithContent[]>([])

// Computed properties
const manifest = computed(() => manifestStore.manifest)
const excludedCount = computed(() => manifestStore.excludedAddons.size)

// Status management
const clearStatus = () =>
{
	statusMessage.value = ''
	statusType.value = 'info'
}

const setStatus = (message: string, type: 'success' | 'error' | 'info' | 'warning') =>
{
	statusMessage.value = message
	statusType.value = type
}

const handleStatus = (message: string, type: 'success' | 'error' | 'info' | 'warning') =>
{
	setStatus(message, type)
}

// Exclusion handling
function handleToggleExclusion(addonName: string)
{
	manifestStore.toggleExclusion(addonName)
}

function clearAllExclusions()
{
	manifestStore.clearExclusions()
}

// UI helper functions for upload button
const getUploadButtonText = () =>
{
	if (manifest.value !== null && selectedConfigFiles.value.length > 0)
	{
		return 'Upload to GitHub'
	}
	else if (manifest.value !== null)
	{
		return 'Upload to GitHub'
	}
	else if (selectedConfigFiles.value.length > 0)
	{
		return 'Upload Config Only'
	}
	return 'Upload to GitHub'
}

const getUploadButtonDescription = () =>
{
	if (uploading.value)
	{
		return 'upload-disabled-help'
	}
	if (manifest.value == null && selectedConfigFiles.value.length === 0)
	{
		return 'upload-disabled-help'
	}
	return 'upload-help'
}

// Action handlers using the composable
async function handleLoadInstance()
{
	clearStatus()
	await loadInstance(setStatus)
}

async function handleSaveManifest()
{
	clearStatus()
	if (manifest.value !== null)
	{
		await saveManifest(manifest.value, setStatus)
	}
}

async function handleUploadToGithub()
{
	if (manifest.value == null && selectedConfigFiles.value.length === 0)
	{
		return
	}

	clearStatus()
	progress.value = 0
	uploading.value = true

	try
	{
		await uploadToGithub(
			manifest.value,
			selectedConfigFiles.value,
			(p: number, msg?: string) =>
			{
				progress.value = p
				if (msg !== undefined) setStatus(msg, 'info')
			},
			setStatus
		)
	}
	finally
	{
		uploading.value = false
		progress.value = 100
	}
}

// Navigation state management
const route = useRoute()

const resetComponentState = () =>
{
	uploading.value = false
	progress.value = 0
	statusMessage.value = ''
	statusType.value = 'info'
	logger.info('AdminPanel state reset after navigation')
}

watch(() => route.name, (newRouteName, oldRouteName) =>
{
	if (oldRouteName === 'settings' && newRouteName === 'dashboard')
	{
		resetComponentState()
		logger.info('Detected navigation from settings to dashboard, AdminPanel state reset')
	}
})

onMounted(() =>
{
	logger.info('AdminPanel mounted')
})

onUnmounted(() =>
{
	logger.info('AdminPanel unmounted')
})
</script>
