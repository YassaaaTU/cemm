<template>
  <div
    v-if="message"
    :class="['alert', alertType]"
  >
    {{ message }}
  </div>
</template>

<script setup lang="ts">
const props = defineProps<{ message: string, type?: 'success' | 'error' | 'info' | 'warning' }>()
const alertType = computed(() =>
{
	switch (props.type)
	{
		case 'success': return 'alert-success'
		case 'error': return 'alert-error'
		case 'info': return 'alert-info'
		case 'warning': return 'alert-warning'
		default: return 'alert-info'
	}
})
</script>
