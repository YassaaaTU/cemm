<template>
  <div
    v-if="shouldShow"
    role="alert"
    class="alert alert-vertical sm:alert-horizontal shadow-lg mb-4"
    :class="alertClass"
  >
    <!-- Alert Icon -->
    <Icon
      v-if="alertType === 'error' || (errorState?.error?.severity === 'critical')"
      name="solar:danger-square-bold"
      class="shrink-0"
      size="1.8rem"
    />
    <Icon
      v-else-if="alertType === 'warning' || (errorState?.error?.severity === 'high')"
      name="solar:shield-warning-bold"
      class="shrink-0"
      size="1.8rem"
    />
    <Icon
      v-else-if="alertType === 'success'"
      name="solar:check-square-bold"
      class="shrink-0"
      size="1.8rem"
    />
    <Icon
      v-else
      name="solar:info-square-bold"
      class="shrink-0"
      size="1.8rem"
    />

    <!-- Alert Content -->
    <div>
      <h3 class="font-bold">
        {{ displayMessage }}
      </h3>

      <div
        v-if="suggestion"
        class="text-xs"
      >
        {{ suggestion }}
      </div>

      <!-- Technical Details (Collapsible) for errors -->
      <details
        v-if="showTechnicalDetails && technicalMessage"
        class="mt-2"
      >
        <summary class="text-xs cursor-pointer opacity-70 hover:opacity-100">
          Technical Details
        </summary>
        <div class="mt-1 p-2 bg-base-300 rounded text-xs font-mono">
          {{ technicalMessage }}
        </div>
      </details>

      <!-- Retry Count for errors -->
      <div
        v-if="errorState?.retryCount && errorState.retryCount > 0"
        class="text-xs opacity-70 mt-1"
      >
        Retry attempt: {{ errorState.retryCount }}
      </div>
    </div>

    <!-- Action Buttons -->
    <div class="flex gap-2">
      <!-- Retry Button (for errors) -->
      <button
        v-if="canRetry && retryOperation !== undefined"
        class="btn btn-sm"
        :class="retryButtonClass"
        :disabled="errorState?.isRetrying"
        @click="handleRetry"
      >
        <span
          v-if="errorState?.isRetrying"
          class="loading loading-spinner loading-xs"
        />
        <span v-else>Retry</span>
      </button>

      <!-- Recovery Action Button (for errors) -->
      <button
        v-if="errorState?.error?.recoveryAction"
        class="btn btn-sm btn-outline"
        @click="errorState.error.recoveryAction"
      >
        Fix Issue
      </button>

      <!-- Close Button -->
      <button
        v-if="showCloseButton"
        class="btn btn-sm btn-ghost btn-square"
        @click="$emit('close')"
      >
        <Icon
          name="solar:close-square-bold"
          size="1.8rem"
          class="shrink-0"
        />
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { ErrorState } from '~/utils/errorHandler'

type AlertType = 'success' | 'error' | 'info' | 'warning'

interface Props
{
	// Simple alert props (from StatusAlert)
	message?: string
	type?: AlertType

	// Error alert props (from ErrorAlert)
	errorState?: ErrorState
	retryOperation?: () => Promise<void>
	showTechnicalDetails?: boolean

	// Common props
	showCloseButton?: boolean
}

interface Emits
{
	close: []
	retry: []
}

const props = withDefaults(defineProps<Props>(), {
	message: '',
	type: 'info',
	errorState: undefined,
	retryOperation: undefined,
	showTechnicalDetails: false,
	showCloseButton: true
})

const emit = defineEmits<Emits>()

// Computed properties
const shouldShow = computed(() =>
{
	return (props.message && props.message.length > 0) || (props.errorState?.error != null)
})

const alertType = computed((): AlertType =>
{
	// If errorState is provided, determine type from error severity
	if (props.errorState?.error != null)
	{
		switch (props.errorState.error.severity)
		{
			case 'critical':
			case 'high':
				return 'error'
			case 'medium':
				return 'warning'
			case 'low':
				return 'info'
			default:
				return 'error'
		}
	}

	// Otherwise use the provided type
	return props.type
})

const alertClass = computed(() =>
{
	switch (alertType.value)
	{
		case 'success':
			return 'alert-success'
		case 'error':
			return 'alert-error'
		case 'warning':
			return 'alert-warning'
		case 'info':
			return 'alert-info'
		default:
			return 'alert-info'
	}
})

const retryButtonClass = computed(() =>
{
	switch (alertType.value)
	{
		case 'error':
			return 'btn-error'
		case 'warning':
			return 'btn-warning'
		case 'info':
			return 'btn-info'
		case 'success':
			return 'btn-success'
		default:
			return 'btn-primary'
	}
})

const displayMessage = computed(() =>
{
	return (props.errorState?.error?.userMessage ?? props.message) || ''
})

const suggestion = computed(() =>
{
	return props.errorState?.error?.suggestion ?? ''
})

const technicalMessage = computed(() =>
{
	return props.errorState?.error?.message ?? ''
})

const canRetry = computed(() =>
{
	return props.errorState?.error?.canRetry ?? false
})

// Methods
const handleRetry = async () =>
{
	if (props.retryOperation != null)
	{
		await props.retryOperation()
	}
	emit('retry')
}
</script>
