<template>
  <Teleport to="body">
    <div
      v-if="updater.isUpdateDialogVisible.value"
      class="modal modal-open z-50"
      role="dialog"
      aria-modal="true"
      aria-labelledby="update-dialog-title"
    >
      <div class="modal-box">
        <!-- Header -->
        <div class="flex items-center gap-3 mb-4">
          <div class="bg-primary/10 text-primary rounded-full p-2">
            <Icon
              name="mdi:update"
              size="1.5em"
            />
          </div>
          <div>
            <h3
              id="update-dialog-title"
              class="text-lg font-semibold"
            >
              Update Available
            </h3>
            <p class="text-sm opacity-70">
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
            class="text-sm opacity-70"
          >
            Download size: {{ updater.formatBytes(updater.updateInfo.value.size) }}
          </div>
          <p class="text-sm opacity-80">
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
        <div class="modal-action">
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
            class="btn"
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
      <div
        class="modal-backdrop"
        aria-hidden="true"
        @click="handleLater"
      />
    </div>
  </Teleport>
</template>

<script setup lang="ts">
const updater = useUpdater()
const { $logger: logger } = useNuxtApp()

const updateError = ref<string | null>(null)

const handleUpdateConfirm = async () =>
{
	updateError.value = null
	try
	{
		logger.info('Starting update download and install process')
		await updater.downloadAndInstall()
		logger.info('Update completed successfully')
	}
	catch (err: unknown)
	{
		logger.error('Update failed', err)
		if (err instanceof Error)
		{
			updateError.value = err.message
		}
		else if (typeof err === 'object' && err !== null)
		{
			try
			{
				updateError.value = JSON.stringify(err)
			}
			catch
			{
				updateError.value = String(err)
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
}

// Handle body scroll lock
watch(() => updater.isUpdateDialogVisible.value, (visible) =>
{
	if (visible)
	{
		logger.info('UpdateDialog: Update available, showing dialog')
		document.body.style.overflow = 'hidden'
	}
	else
	{
		document.body.style.overflow = ''
	}
}, { immediate: true })

watch(() => updater.updateInfo.value, (updateInfo) =>
{
	logger.info('UpdateDialog: Update info changed', updateInfo)
}, { immediate: true })
</script>
