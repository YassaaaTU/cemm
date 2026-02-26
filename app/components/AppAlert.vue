<template>
  <div
    v-if="shouldShow"
    role="alert"
    class="alert alert-vertical sm:alert-horizontal shadow-lg mb-4"
    :class="alertClass"
  >
    <!-- Alert Icon -->
    <Icon
      v-if="type === 'error'"
      name="solar:danger-square-bold"
      class="shrink-0"
      size="1.8rem"
    />
    <Icon
      v-else-if="type === 'warning'"
      name="solar:shield-warning-bold"
      class="shrink-0"
      size="1.8rem"
    />
    <Icon
      v-else-if="type === 'success'"
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
        {{ message }}
      </h3>

      <div
        v-if="suggestion"
        class="text-xs"
      >
        {{ suggestion }}
      </div>
    </div>

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
</template>

<script setup lang="ts">
type AlertType = 'success' | 'error' | 'info' | 'warning'

interface Props
{
	message?: string
	type?: AlertType
	suggestion?: string
	showCloseButton?: boolean
}

const props = withDefaults(defineProps<Props>(), {
	message: '',
	type: 'info',
	suggestion: undefined,
	showCloseButton: true
})

defineEmits<{
	close: []
}>()

// Computed properties
const shouldShow = computed(() =>
{
	return props.message && props.message.length > 0
})

const alertClass = computed(() =>
{
	switch (props.type)
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
</script>
