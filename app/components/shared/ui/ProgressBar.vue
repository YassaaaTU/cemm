<template>
  <div class="w-full">
    <progress
      class="progress w-full transition-all duration-300"
      :class="[colorClass, sizeClass]"
      :value="indeterminate ? undefined : normalizedProgress"
      max="100"
      :aria-valuenow="indeterminate ? undefined : Math.round(normalizedProgress)"
      :aria-label="indeterminate ? 'Progress in progress' : `${Math.round(normalizedProgress)}% complete`"
    />

    <div
      v-if="label.length > 0 || (showPercentage && !indeterminate)"
      class="mt-2 flex items-center gap-2"
      :class="label.length > 0 && showPercentage && !indeterminate ? 'justify-between' : 'justify-end'"
    >
      <span
        v-if="label.length > 0"
        class="text-sm opacity-80"
      >
        {{ label }}
      </span>
      <span
        v-if="showPercentage && !indeterminate"
        class="text-sm font-semibold tabular-nums"
      >
        {{ Math.round(normalizedProgress) }}%
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
interface Props
{
	progress: number
	label?: string
	showPercentage?: boolean
	indeterminate?: boolean
	color?: 'primary' | 'secondary' | 'accent' | 'success' | 'warning' | 'error' | 'info'
	size?: 'xs' | 'sm' | 'md' | 'lg'
}

const props = withDefaults(defineProps<Props>(), {
	label: '',
	showPercentage: true,
	indeterminate: false,
	color: 'primary',
	size: 'md'
})

// Tailwind 4 scans source text for literal class names — a template-literal
// class like `progress-${color}` is invisible to it, so every variant is
// dropped from the build except whichever one happens to appear literally
// elsewhere (see F-P2-5). Mapping through a static object keeps every class
// name literal in this file.
const colorClassMap: Record<NonNullable<Props['color']>, string> = {
	primary: 'progress-primary',
	secondary: 'progress-secondary',
	accent: 'progress-accent',
	success: 'progress-success',
	warning: 'progress-warning',
	error: 'progress-error',
	info: 'progress-info'
}

const colorClass = computed(() => colorClassMap[props.color])

const sizeClass = computed(() =>
{
	if (props.size === 'xs')
	{
		return 'h-1'
	}

	if (props.size === 'sm')
	{
		return 'h-2'
	}

	if (props.size === 'lg')
	{
		return 'h-4'
	}

	return 'h-3'
})

const normalizedProgress = computed(() =>
{
	if (!Number.isFinite(props.progress))
	{
		return 0
	}

	if (props.progress < 0)
	{
		return 0
	}

	if (props.progress > 100)
	{
		return 100
	}

	return props.progress
})
</script>
