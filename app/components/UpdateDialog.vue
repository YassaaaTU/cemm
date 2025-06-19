<!-- components/UpdateDialog.vue -->
<template>
  <div
    v-if="isVisible"
    class="fixed inset-0 z-50 flex items-center justify-center"
  >
    <!-- Backdrop -->
    <div
      class="fixed inset-0 bg-black bg-opacity-50"
      @click="handleCancel"
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
            {{ title }}
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
            <span class="badge badge-outline">{{ currentVersion }}</span>
          </div>
          <div class="flex justify-between items-center">
            <span class="text-sm font-medium">New Version:</span>
            <span class="badge badge-primary">{{ newVersion }}</span>
          </div>
        </div>

        <p class="text-sm text-base-content/80">
          {{ message }}
        </p>

        <!-- Progress bar (when downloading) -->
        <div
          v-if="isDownloading"
          class="space-y-2"
        >
          <div class="flex justify-between text-sm">
            <span>Downloading update...</span>
            <span>{{ downloadProgress }}%</span>
          </div>
          <progress
            class="progress progress-primary w-full"
            :value="downloadProgress"
            max="100"
          />
        </div>
      </div>

      <!-- Actions -->
      <div class="flex gap-3 justify-end">
        <button
          v-if="!isDownloading"
          class="btn btn-ghost"
          @click="handleCancel"
        >
          Later
        </button>
        <button
          v-if="!isDownloading"
          class="btn btn-primary"
          @click="handleConfirm"
        >
          <Icon
            name="mdi:download"
            size="1.4em"
          />
          Update Now
        </button>
        <button
          v-if="isDownloading"
          class="btn btn-disabled"
          disabled
        >
          <Icon
            name="line-md:loading-loop"
            size="1.4em"
          />
          Installing...
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
interface Props
{
	isVisible: boolean
	title: string
	message: string
	currentVersion: string
	newVersion: string
	isDownloading?: boolean
	downloadProgress?: number
}

interface Emits
{
	confirm: []
	cancel: []
}

const props = withDefaults(defineProps<Props>(), {
	isDownloading: false,
	downloadProgress: 0
})

const emit = defineEmits<Emits>()

const handleConfirm = () =>
{
	emit('confirm')
}

const handleCancel = () =>
{
	emit('cancel')
}

// Prevent body scroll when dialog is open
watch(() => props.isVisible, (visible) =>
{
	if (visible)
	{
		document.body.style.overflow = 'hidden'
	}
	else
	{
		document.body.style.overflow = ''
	}
})
</script>
