<!-- components/UpdateDialog.vue -->
<template>
  <div
    v-if="updater.isUpdateDialogVisible.value"
    class="fixed inset-0 z-50 flex items-center justify-center"
  >
    <!-- Backdrop -->
    <div
      class="fixed inset-0 bg-black bg-opacity-50"
      @click="handleLater"
    />

    <!-- Dialog -->
    <div class="relative bg-base-100 rounded-lg shadow-xl max-w-md w-full mx-4 p-6">
      <!-- Header -->
      <div class="flex items-center gap-3 mb-4">
        <div class="avatar avatar-online bg-primary/10 text-white rounded-full p-2">
          <Icon
            name="mdi:update"
            size="1.5em"
            class="text-primary"
          />
        </div>
        <div>
          <h3 class="text-lg font-semibold">
            Update Available
          </h3>
          <p class="text-sm text-base-content/70">
            CEMM Update Available
          </p>
        </div>
      </div>

      <!-- Content -->
      <div class="mb-6 space-y-3">
        <div class="bg-base-200 rounded-lg p-4">
          <div class="flex justify-between items-center mb-2">
            <span class="text-sm font-medium">Current Version:</span>
            <span class="badge badge-outline">{{ updater.updateInfo.value?.current_version }}</span>
          </div>
          <div class="flex justify-between items-center">
            <span class="text-sm font-medium">New Version:</span>
            <span class="badge badge-primary">{{ updater.updateInfo.value?.latest_version }}</span>
          </div>
        </div>
        <div
          v-if="updater.updateInfo.value?.size"
          class="text-sm text-base-content/70"
        >
          Download size: {{ updater.formatBytes(updater.updateInfo.value.size) }}
        </div>
        <p class="text-sm text-base-content/80">
          A new version of CEMM is available. Click "Update Now" to download and install automatically.
        </p>

        <!-- Error Alert -->
        <div
          v-if="updateError"
          class="alert alert-error"
        >
          <Icon name="mdi:alert-circle" />
          <span>{{ updateError }}</span>
        </div>

        <!-- Progress bar (when downloading) -->
        <div
          v-if="updater.isDownloading.value"
          class="space-y-2"
        >
          <div class="flex justify-between text-sm">
            <span>Downloading update...</span>
            <span>{{ updater.downloadProgress.value }}%</span>
          </div>
          <progress
            class="progress progress-primary w-full"
            :value="updater.downloadProgress.value"
            max="100"
          />
        </div>

        <!-- Installing state -->
        <div
          v-if="updater.isInstalling.value"
          class="space-y-2"
        >
          <div class="flex items-center gap-2 text-sm">
            <span class="loading loading-spinner loading-sm" />
            <span>Installing update...</span>
          </div>
        </div>
      </div>

      <!-- Actions -->
      <div class="flex gap-3 justify-end">
        <button
          v-if="!updater.isDownloading.value && !updater.isInstalling.value"
          class="btn btn-ghost"
          @click="handleLater"
        >
          Later
        </button>
        <button
          v-if="!updater.isDownloading.value && !updater.isInstalling.value"
          class="btn btn-primary"
          :disabled="!updater.updateInfo.value?.available"
          @click="handleUpdateConfirm"
        >
          <Icon
            name="mdi:download"
            size="1.4em"
          />
          Update Now
        </button>
        <button
          v-if="updater.isDownloading.value || updater.isInstalling.value"
          class="btn btn-disabled"
          disabled
        >
          <Icon
            name="line-md:loading-loop"
            size="1.4em"
          />
          {{ updater.isDownloading.value ? 'Downloading...' : 'Installing...' }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const updater = useUpdater()
// const { $logger } = useNuxtApp()

const updateError = ref<string | null>(null)

const handleUpdateConfirm = async () =>
{
	updateError.value = null
	try
	{
		console.info('🚀 Starting update download and install process')
		await updater.downloadAndInstall()
		console.info('✅ Update completed successfully')
	}
	catch (err: unknown)
	{
		console.error('❌ Update failed:')
		console.error(err)
		// Log JSON representation if possible
		try
		{
			console.error('Update error (JSON):', JSON.stringify(err, null, 2))
		}
		catch (_)
		{ /* empty */ }
		// Show both string and JSON error in UI for diagnostics
		if (err instanceof Error)
		{
			updateError.value = err.message
			console.error('Error message:', err.message)
			console.error('Error stack:', err.stack)
		}
		else if (typeof err === 'object' && err !== null)
		{
			try
			{
				updateError.value = JSON.stringify(err)
				console.error('Error object:', err)
			}
			catch
			{
				updateError.value = String(err)
				console.error('Error (string):', String(err))
			}
		}
		else
		{
			updateError.value = String(err)
		}
	}
}

const handleLater = () =>
{
	updater.handleUpdateCancel()
	// Optionally reset progress state here if needed
}

// Debug logging for dialog visibility
watch(() => updater.isUpdateDialogVisible.value, (visible) =>
{
	console.info('UpdateDialog visibility changed:', { visible })

	if (visible)
	{
		console.info('UpdateDialog: Update available, showing dialog')
		console.info('UpdateDialog: Update info:', updater.updateInfo.value)
		document.body.style.overflow = 'hidden'
	}
	else
	{
		console.info('UpdateDialog: Hiding dialog')
		document.body.style.overflow = ''
	}
}, { immediate: true })

// Debug logging for update info changes
watch(() => updater.updateInfo.value, (updateInfo) =>
{
	console.info('UpdateDialog: Update info changed:', updateInfo)
}, { immediate: true })
</script>
