<template>
  <div class="progress-container">
    <!-- Enhanced progress bar with animations -->
    <div class="progress-wrapper">
      <progress
        class="progress w-full"
        :class="progressClass"
        :value="progress"
        max="100"
      />

      <!-- Progress text overlay -->
      <div
        v-if="showPercentage"
        class="progress-text"
      >
        {{ Math.round(progress) }}%
      </div>
    </div>

    <!-- Optional label/message -->
    <div
      v-if="label"
      class="progress-label"
    >
      {{ label }}
    </div>

    <!-- Animated pulse effect for indeterminate progress -->
    <div
      v-if="indeterminate"
      class="progress-pulse"
    >
      <div class="pulse-bar" />
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

const progressClass = computed(() =>
{
	const classes = []

	// Color classes
	if (props.color !== 'primary')
	{
		classes.push(`progress-${props.color}`)
	}

	// Size classes
	if (props.size !== 'md')
	{
		classes.push(`progress-${props.size}`)
	}

	return classes.join(' ')
})
</script>

<style scoped>
.progress-container
{
	position: relative;
	width: 100%;
}

.progress-wrapper
{
	position: relative;
	width: 100%;
}

.progress-text
{
	position: absolute;
	top: 50%;
	left: 50%;
	transform: translate(-50%, -50%);
	font-size: 0.75rem;
	font-weight: 600;
	color: var(--fallback-bc, oklch(var(--bc)/1));
	text-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
	pointer-events: none;
}

.progress-label
{
	margin-top: 0.5rem;
	font-size: 0.875rem;
	text-align: center;
	opacity: 0.8;
}

.progress-pulse
{
	position: absolute;
	top: 0;
	left: 0;
	right: 0;
	bottom: 0;
	overflow: hidden;
	border-radius: inherit;
}

.pulse-bar
{
	position: absolute;
	top: 0;
	left: -100%;
	width: 100%;
	height: 100%;
	background: linear-gradient(
		90deg,
		transparent,
		rgba(255, 255, 255, 0.4),
		transparent
	);
	animation: pulse-slide 2s infinite;
}

@keyframes pulse-slide
{
	0%
	{
		left: -100%;
	}
	100%
	{
		left: 100%;
	}
}

/* Enhanced progress bar styles */
.progress
{
	transition: all 0.3s ease;
}

.progress::-webkit-progress-bar
{
	background-color: rgba(255, 255, 255, 0.1);
	border-radius: 0.5rem;
}

.progress::-webkit-progress-value
{
	background: linear-gradient(90deg, var(--fallback-p, oklch(var(--p)/1)), var(--fallback-s, oklch(var(--s)/1)));
	border-radius: 0.5rem;
	transition: width 0.3s ease;
}

.progress::-moz-progress-bar
{
	background: linear-gradient(90deg, var(--fallback-p, oklch(var(--p)/1)), var(--fallback-s, oklch(var(--s)/1)));
	border-radius: 0.5rem;
	transition: width 0.3s ease;
}

/* Size variants */
.progress-xs
{
	height: 0.25rem;
}

.progress-sm
{
	height: 0.5rem;
}

.progress-lg
{
	height: 1rem;
}

/* Color variants */
.progress-success::-webkit-progress-value,
.progress-success::-moz-progress-bar
{
	background: linear-gradient(90deg, var(--fallback-su, oklch(var(--su)/1)), #10b981);
}

.progress-warning::-webkit-progress-value,
.progress-warning::-moz-progress-bar
{
	background: linear-gradient(90deg, var(--fallback-wa, oklch(var(--wa)/1)), #f59e0b);
}

.progress-error::-webkit-progress-value,
.progress-error::-moz-progress-bar
{
	background: linear-gradient(90deg, var(--fallback-er, oklch(var(--er)/1)), #ef4444);
}

.progress-info::-webkit-progress-value,
.progress-info::-moz-progress-bar
{
	background: linear-gradient(90deg, var(--fallback-in, oklch(var(--in)/1)), #3b82f6);
}
</style>
