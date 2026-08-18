<template>
  <!-- Native <dialog> so focus
       trapping, Escape and the top layer come from the platform rather than
       from a hand-rolled modal wrapper. -->
  <dialog
    ref="dialogRef"
    class="modal modal-bottom sm:modal-middle"
    aria-labelledby="update-dialog-title"
    @close="handleNativeClose"
  >
    <div class="modal-box max-w-md border border-base-300 bg-base-200 p-0">
      <div class="flex items-center gap-2.5 border-b border-base-300 px-4 py-3">
        <Icon
          name="mdi:package-down"
          size="1.25rem"
          class="shrink-0 text-primary"
          aria-hidden="true"
        />
        <div class="min-w-0">
          <h3
            id="update-dialog-title"
            class="text-base font-bold"
          >
            New version available
          </h3>
        </div>
      </div>

      <div class="space-y-3 p-4 ">
        <!-- Version change, read as a route: what you are on, what you go to. -->
        <div class="flex items-center gap-3 rounded-box border border-base-300 bg-base-100 px-3 py-2.5">
          <div class="min-w-0">
            <p class="text-xs text-base-content/55">
              Installed
            </p>
            <p class="font-mono text-base text-base-content/70 tabular-nums">
              {{ updater.updateInfo.value?.currentVersion ?? '—' }}
            </p>
          </div>

          <Icon
            name="mdi:arrow-right"
            size="1.1rem"
            class="shrink-0 text-primary"
            aria-hidden="true"
          />

          <div class="min-w-0">
            <p class="text-xs text-base-content/55">
              Available
            </p>
            <p class="font-mono text-base font-bold text-primary tabular-nums">
              {{ updater.updateInfo.value?.version ?? '—' }}
            </p>
          </div>
        </div>

        <p class="text-sm/relaxed  text-base-content/70">
          CEMM will download the update and restart itself. Your settings and
          saved repository are kept.
        </p>

        <div
          v-if="updateError !== null"
          role="alert"
          class="alert alert-soft text-sm alert-error"
        >
          <Icon
            name="mdi:alert-circle-outline"
            size="1.1rem"
            aria-hidden="true"
          />
          <span class="min-w-0 wrap-break-word">{{ updateError }}</span>
        </div>

        <div
          v-if="updater.isDownloading.value"
          class="space-y-1.5"
        >
          <div class="flex items-baseline justify-between gap-2 text-xs">
            <span class="font-semibold">Downloading…</span>
            <span class="font-mono text-primary tabular-nums">
              {{ updater.downloadProgress.value }}%
              <template v-if="updater.totalBytes.value > 0">
                ({{ updater.formatBytes(updater.downloadedBytes.value) }} / {{ updater.formatBytes(updater.totalBytes.value) }})
              </template>
            </span>
          </div>
          <progress
            class="progress w-full"
            :value="updater.downloadProgress.value"
            max="100"
            aria-label="Download progress"
          />
        </div>

        <div
          v-if="updater.isInstalling.value"
          class="flex items-center gap-2 text-sm"
        >
          <span
            class="loading loading-sm loading-spinner"
            aria-hidden="true"
          />
          <span>Installing — CEMM will restart.</span>
        </div>
      </div>

      <div class="modal-action mt-0 flex gap-2 border-t border-base-300 px-4 py-3">
        <template v-if="!updater.isDownloading.value && !updater.isInstalling.value">
          <button
            type="button"
            class="btn btn-ghost btn-sm"
            @click="handleLater"
          >
            Not now
          </button>
          <button
            type="button"
            class="btn gap-1.5 btn-primary btn-sm"
            :disabled="updater.updateInfo.value === null"
            @click="handleUpdateConfirm"
          >
            <Icon
              name="mdi:download"
              size="1rem"
              aria-hidden="true"
            />
            Update and restart
          </button>
        </template>
        <button
          v-else
          type="button"
          class="btn btn-sm"
          disabled
        >
          <span
            class="loading loading-xs loading-spinner"
            aria-hidden="true"
          />
          {{ updater.isDownloading.value ? 'Downloading…' : 'Installing…' }}
        </button>
      </div>
    </div>

    <!-- No backdrop dismissal while an install is in flight: closing the dialog
         mid-download would hide the only progress the user can see. -->
    <form
      v-if="!updater.isDownloading.value && !updater.isInstalling.value"
      method="dialog"
      class="modal-backdrop"
    >
      <button>Close</button>
    </form>
  </dialog>
</template>

<script setup lang="ts">
const updater = useUpdater()
const { $logger: logger } = useNuxtApp()

const dialogRef = ref<HTMLDialogElement | null>(null)
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
		logger.error({ error: err }, 'Update failed')
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
	dialogRef.value?.close()
}

const handleNativeClose = () =>
{
	// Escape and backdrop clicks route through here too, so the updater state
	// stays in step however the dialog was dismissed.
	updater.handleUpdateCancel()
}

watch(() => updater.isUpdateDialogVisible.value, (visible) =>
{
	const dialog = dialogRef.value
	if (dialog === null) return

	if (visible)
	{
		logger.info('UpdateDialog: Update available, showing dialog')
		if (!dialog.open) dialog.showModal()
	}
	else if (dialog.open)
	{
		dialog.close()
	}
}, { immediate: true })

watch(() => updater.updateInfo.value, (updateInfo) =>
{
	logger.info({ updateInfo }, 'UpdateDialog: Update info changed')
}, { immediate: true })
</script>
