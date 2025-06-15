<template>
  <div
    v-if="errorState.error"
    class="alert shadow-lg mb-4"
    :class="alertClass"
  >
    <div class="flex-1">
      <div class="flex items-start gap-3">
        <!-- Error Icon -->
        <div class="flex-shrink-0">
          <svg
            v-if="errorState.error.severity === 'critical'"
            class="w-6 h-6"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M12 9v2m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
            />
          </svg>
          <svg
            v-else-if="errorState.error.severity === 'high'"
            class="w-6 h-6"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
            />
          </svg>
          <svg
            v-else
            class="w-6 h-6"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
            />
          </svg>
        </div>

        <!-- Error Content -->
        <div class="flex-1">
          <h3 class="font-semibold">
            {{ errorState.error.userMessage }}
          </h3>

          <p
            v-if="errorState.error.suggestion"
            class="text-sm opacity-80 mt-1"
          >
            {{ errorState.error.suggestion }}
          </p>

          <!-- Technical Details (Collapsible) -->
          <details
            v-if="showTechnicalDetails && errorState.error.message"
            class="mt-2"
          >
            <summary class="text-sm cursor-pointer opacity-70 hover:opacity-100">
              Technical Details
            </summary>
            <div class="mt-1 p-2 bg-base-300 rounded text-sm font-mono">
              {{ errorState.error.message }}
            </div>
          </details>

          <!-- Retry Count -->
          <div
            v-if="errorState.retryCount > 0"
            class="text-sm opacity-70 mt-1"
          >
            Retry attempt: {{ errorState.retryCount }}
          </div>
        </div>
      </div>
    </div>

    <!-- Action Buttons -->
    <div class="flex-none">
      <div class="flex gap-2">
        <!-- Retry Button -->
        <button
          v-if="errorState.error.canRetry && retryOperation !== undefined"
          class="btn btn-sm"
          :class="retryButtonClass"
          :disabled="errorState.isRetrying"
          @click="handleRetry"
        >
          <span
            v-if="errorState.isRetrying"
            class="loading loading-spinner loading-xs"
          />
          <span v-else>Retry</span>
        </button>

        <!-- Recovery Action Button -->
        <button
          v-if="errorState.error.recoveryAction"
          class="btn btn-sm btn-outline"
          @click="errorState.error.recoveryAction"
        >
          Fix Issue
        </button>

        <!-- Close Button -->
        <button
          class="btn btn-sm btn-ghost"
          @click="$emit('close')"
        >
          <svg
            class="w-4 h-4"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M6 18L18 6M6 6l12 12"
            />
          </svg>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { ErrorState } from '~/utils/errorHandler'

interface Props
{
	errorState: ErrorState
	retryOperation?: () => Promise<void>
	showTechnicalDetails?: boolean
}

interface Emits
{
	close: []
	retry: []
}

const props = withDefaults(defineProps<Props>(), {
	retryOperation: undefined,
	showTechnicalDetails: false
})

const emit = defineEmits<Emits>()

const alertClass = computed(() =>
{
	if (props.errorState.error === null) return ''

	switch (props.errorState.error.severity)
	{
		case 'critical':
			return 'alert-error'
		case 'high':
			return 'alert-error'
		case 'medium':
			return 'alert-warning'
		case 'low':
			return 'alert-info'
		default:
			return 'alert-warning'
	}
})

const retryButtonClass = computed(() =>
{
	if (props.errorState.error === null) return ''

	switch (props.errorState.error.severity)
	{
		case 'critical':
		case 'high':
			return 'btn-error'
		case 'medium':
			return 'btn-warning'
		case 'low':
			return 'btn-info'
		default:
			return 'btn-primary'
	}
})

const handleRetry = async () =>
{
	if (props.retryOperation !== undefined)
	{
		await props.retryOperation()
	}
	emit('retry')
}
</script>
