<template>
  <div
    class="admin-panel"
    role="region"
    aria-labelledby="admin-mode-title"
  >
    <h1
      id="admin-mode-title"
      class="sr-only"
    >
      Admin Mode
    </h1>

    <!-- Zone 1: Command Bar -->
    <CommandBar
      :custom-modpack-name="customModpackName"
      :manifest-loaded="manifest != null"
      :uploading="uploading"
      :config-files-count="selectedConfigFiles.length"
      :excluded-count="excludedCount"
      @update:custom-modpack-name="customModpackName = $event"
      @load-instance="handleLoadInstance"
      @save-manifest="handleSaveManifest"
      @upload-to-github="handleUploadToGithub"
      @clear-exclusions="clearAllExclusions"
    />

    <!-- Progress Bar -->
    <div
      v-if="uploading || progress > 0"
      class="admin-panel__progress"
    >
      <ProgressBar
        :progress="progress"
        :label="uploading ? 'Uploading to GitHub...' : ''"
        :color="uploading ? 'primary' : 'success'"
        :show-percentage="uploading"
      />
    </div>

    <!-- Status Alert -->
    <div
      v-if="statusMessage"
      class="alert"
      :class="`alert-${statusType}`"
      role="alert"
    >
      <Icon
        :name="alertIcon"
        size="1.4rem"
        class="shrink-0"
      />
      <span>{{ statusMessage }}</span>
      <button
        class="btn btn-sm btn-ghost"
        aria-label="Dismiss status message"
        @click="clearStatus"
      >
        <Icon
          name="mdi:close"
          size="1.2rem"
        />
      </button>
    </div>

    <!-- Update Reference (after upload) -->
    <div
      v-if="latestUpdateReference.length > 0"
      class="card bg-base-200 shadow-md border border-base-300 admin-panel__ref-card"
    >
      <div class="card-body">
        <div class="admin-panel__ref">
          <div class="admin-panel__ref-info">
            <Icon
              name="mdi:check-circle"
              size="1.2rem"
              class="admin-panel__ref-icon"
            />
            <div>
              <p class="admin-panel__ref-label">
                Update published successfully
              </p>
              <p class="admin-panel__ref-value">
                Share this update ID with users:
              </p>
              <code class="admin-panel__ref-code">{{ latestUpdateReference }}</code>
            </div>
          </div>
          <button
            class="btn btn-outline btn-sm"
            @click="copyUpdateReference"
          >
            <Icon
              name="mdi:content-copy"
              size="1.1rem"
            />
            Copy ID
          </button>
        </div>
      </div>
    </div>

    <!-- Zone 2: Manifest Display -->
    <div
      v-if="manifest"
      class="admin-panel__manifest"
    >
      <AddonList
        :addons="manifest.mods"
        :update-info="manifestStore.updateInfo"
        :excluded-addons="manifestStore.excludedAddons"
        :show-exclusion="true"
        :show-selection="true"
        title="Mods"
        category="mods"
        class="admin-panel__addon-section"
        @toggle-exclusion="handleToggleExclusion"
        @bulk-actions="handleBulkAction"
      />
      <AddonList
        v-if="manifest.resourcepacks.length > 0"
        :addons="manifest.resourcepacks"
        :update-info="manifestStore.updateInfo"
        :excluded-addons="manifestStore.excludedAddons"
        :show-exclusion="true"
        :show-selection="true"
        title="Resource Packs"
        category="resourcepacks"
        class="admin-panel__addon-section"
        @toggle-exclusion="handleToggleExclusion"
        @bulk-actions="handleBulkAction"
      />
      <AddonList
        v-if="manifest.shaderpacks.length > 0"
        :addons="manifest.shaderpacks"
        :update-info="manifestStore.updateInfo"
        :excluded-addons="manifestStore.excludedAddons"
        :show-exclusion="true"
        :show-selection="true"
        title="Shader Packs"
        category="shaderpacks"
        class="admin-panel__addon-section"
        @toggle-exclusion="handleToggleExclusion"
        @bulk-actions="handleBulkAction"
      />
      <AddonList
        v-if="manifest.datapacks.length > 0"
        :addons="manifest.datapacks"
        :update-info="manifestStore.updateInfo"
        :excluded-addons="manifestStore.excludedAddons"
        :show-exclusion="true"
        :show-selection="true"
        title="Data Packs"
        category="datapacks"
        class="admin-panel__addon-section"
        @toggle-exclusion="handleToggleExclusion"
        @bulk-actions="handleBulkAction"
      />
    </div>

    <!-- Empty State -->
    <div
      v-else
      class="admin-panel__empty"
    >
      <Icon
        name="mdi:folder-search"
        size="3rem"
        class="admin-panel__empty-icon"
      />
      <h3 class="admin-panel__empty-title">
        No Instance Loaded
      </h3>
      <p class="admin-panel__empty-desc">
        Load a Minecraft instance to preview your mods, resource packs, and shader packs.
      </p>
      <button
        class="btn btn-primary"
        @click="handleLoadInstance"
      >
        <Icon
          name="mdi:folder-open"
          size="1.1rem"
        />
        Browse for Instance
      </button>
    </div>

    <!-- Zone 3: Config Files -->
    <ConfigFilesSection
      v-model="selectedConfigFiles"
      class="admin-panel__config"
      @status="handleStatus"
    />
  </div>
</template>

<script setup lang="ts">
import type { ConfigFileWithContent } from '~/types'

const { loadInstance, saveManifest, uploadToGithub } = useAdminApi()
const manifestStore = useManifestStore()
const { $logger: logger } = useNuxtApp()

const uploading = ref(false)
const progress = ref(0)
const statusMessage = ref('')
const statusType = ref<'success' | 'error' | 'info' | 'warning'>('info')
const selectedConfigFiles = ref<ConfigFileWithContent[]>([])
const customModpackName = ref('')
const latestUpdateReference = ref('')

const manifest = computed(() => manifestStore.manifest)
const excludedCount = computed(() => manifestStore.excludedAddons.length)

const alertIcon = computed(() =>
{
	const icons: Record<string, string> = {
		success: 'mdi:check-circle',
		error: 'mdi:alert-circle',
		warning: 'mdi:alert',
		info: 'mdi:information'
	}
	return icons[statusType.value] ?? 'mdi:information'
})

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

function handleToggleExclusion(addonName: string)
{
	manifestStore.toggleExclusion(addonName)
}

function handleBulkAction(_action: 'exclude', addonNames: string[])
{
	for (const addonName of addonNames)
	{
		if (!manifestStore.isExcluded(addonName))
		{
			manifestStore.toggleExclusion(addonName)
		}
	}
}

function clearAllExclusions()
{
	manifestStore.clearExclusions()
}

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
	latestUpdateReference.value = ''
	progress.value = 0
	uploading.value = true

	try
	{
		const result: { success: boolean, updateReference?: string } = await uploadToGithub(
			manifest.value,
			selectedConfigFiles.value,
			customModpackName.value,
			(p: number, msg?: string) =>
			{
				progress.value = p
				if (msg !== undefined) setStatus(msg, 'info')
			},
			setStatus
		)
		if (result.success && typeof result.updateReference === 'string')
		{
			latestUpdateReference.value = result.updateReference
			progress.value = 100
		}
	}
	finally
	{
		uploading.value = false
	}
}

async function copyUpdateReference()
{
	if (latestUpdateReference.value.length === 0)
	{
		return
	}

	try
	{
		await navigator.clipboard.writeText(latestUpdateReference.value)
		setStatus('Update ID copied to clipboard.', 'success')
	}
	catch
	{
		setStatus('Failed to copy update id. Please copy it manually.', 'warning')
	}
}

const route = useRoute()

const resetComponentState = () =>
{
	uploading.value = false
	progress.value = 0
	statusMessage.value = ''
	statusType.value = 'info'
	latestUpdateReference.value = ''
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

<style scoped>
.admin-panel {
  max-width: var(--container-lg);
  width: 100%;
  margin: 0 auto;
}

.admin-panel__progress {
  margin-bottom: var(--space-4);
}

.admin-panel__ref-card {
  margin-bottom: var(--space-6);
}

.admin-panel__ref {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
  flex-wrap: wrap;
}

.admin-panel__ref-info {
  display: flex;
  align-items: flex-start;
  gap: var(--space-3);
}

.admin-panel__ref-icon {
  color: var(--color-status-success);
}

.admin-panel__ref-label {
  font-weight: 600;
  font-size: var(--text-body-md-size);
  color: var(--color-text-primary);
  margin: 0;
}

.admin-panel__ref-value {
  font-size: var(--text-body-sm-size);
  color: var(--color-text-secondary);
  margin: var(--space-1) 0 0;
}

.admin-panel__ref-code {
  display: inline-block;
  margin-top: var(--space-1);
  padding: var(--space-1) var(--space-2);
  background: var(--color-surface-overlay);
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-sm);
  font-family: var(--font-mono);
  font-size: var(--text-body-sm-size);
  color: var(--color-accent-primary);
  word-break: break-all;
}

.admin-panel__manifest {
  margin-bottom: var(--space-6);
}

.admin-panel__addon-section {
  margin-bottom: var(--space-6);
}

.admin-panel__empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--space-16) var(--space-8);
  text-align: center;
  background: var(--color-surface-raised);
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-lg);
  margin-bottom: var(--space-6);
}

.admin-panel__empty-icon {
  color: var(--color-text-tertiary);
  margin-bottom: var(--space-4);
}

.admin-panel__empty-title {
  font-family: var(--font-heading);
  font-size: var(--text-heading-md-size);
  font-weight: 700;
  color: var(--color-text-primary);
  margin: 0 0 var(--space-2);
}

.admin-panel__empty-desc {
  font-size: var(--text-body-md-size);
  color: var(--color-text-secondary);
  margin: 0 0 var(--space-6);
  max-width: 400px;
}

.admin-panel__config {
  margin-top: var(--space-6);
}
</style>
