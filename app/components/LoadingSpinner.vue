<template>
  <div
    v-if="loading"
    class="loading-container"
    :class="containerClass"
  >
    <!-- Spinner with customizable size and style -->
    <div
      class="loading"
      :class="spinnerClass"
    />

    <!-- Optional loading message -->
    <div
      v-if="message"
      class="loading-message"
      :class="messageClass"
    >
      {{ message }}
    </div>

    <!-- Optional progress bar for operations with known progress -->
    <div
      v-if="showProgress && progress !== undefined"
      class="loading-progress"
    >
      <div class="progress-bar">
        <div
          class="progress-fill"
          :style="{ width: `${Math.min(100, Math.max(0, progress))}%` }"
        />
      </div>
      <div class="progress-text">
        {{ Math.round(progress) }}%
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
interface Props
{
	loading: boolean
	message?: string
	progress?: number
	showProgress?: boolean
	size?: 'xs' | 'sm' | 'md' | 'lg' | 'xl'
	variant?: 'spinner' | 'dots' | 'ring' | 'ball' | 'bars' | 'infinity'
	overlay?: boolean
	center?: boolean
}

const props = withDefaults(defineProps<Props>(), {
	message: '',
	progress: undefined,
	showProgress: false,
	size: 'md',
	variant: 'spinner',
	overlay: false,
	center: true
})

// Computed classes for different spinner styles and sizes
const spinnerClass = computed(() =>
{
	const classes = []

	// Base loading class from daisyUI
	classes.push(`loading-${props.variant}`)

	// Size classes
	switch (props.size)
	{
		case 'xs':
			classes.push('loading-xs')
			break
		case 'sm':
			classes.push('loading-sm')
			break
		case 'md':
			classes.push('loading-md')
			break
		case 'lg':
			classes.push('loading-lg')
			break
		case 'xl':
			classes.push('loading-xl')
			break
	}

	return classes.join(' ')
})

const containerClass = computed(() =>
{
	const classes = []

	if (props.overlay)
	{
		classes.push('loading-overlay')
	}

	if (props.center)
	{
		classes.push('loading-centered')
	}

	return classes.join(' ')
})

const messageClass = computed(() =>
{
	const classes = ['text-sm', 'opacity-70', 'mt-2']

	if (props.center)
	{
		classes.push('text-center')
	}

	return classes.join(' ')
})
</script>

<style scoped>
.loading-container
{
	display: flex;
	flex-direction: column;
	align-items: center;
	gap: 0.5rem;
}

.loading-centered
{
	justify-content: center;
	align-items: center;
	text-align: center;
}

.loading-overlay
{
	position: absolute;
	top: 0;
	left: 0;
	right: 0;
	bottom: 0;
	background: rgba(0, 0, 0, 0.1);
	backdrop-filter: blur(2px);
	z-index: 10;
	display: flex;
	flex-direction: column;
	justify-content: center;
	align-items: center;
}

.loading-progress
{
	width: 100%;
	max-width: 200px;
	display: flex;
	flex-direction: column;
	gap: 0.25rem;
}

.progress-bar
{
	width: 100%;
	height: 0.5rem;
	background: rgba(255, 255, 255, 0.2);
	border-radius: 0.25rem;
	overflow: hidden;
}

.progress-fill
{
	height: 100%;
	background: linear-gradient(90deg, #06b6d4, #3b82f6);
	border-radius: 0.25rem;
	transition: width 0.3s ease;
}

.progress-text
{
	font-size: 0.75rem;
	text-align: center;
	opacity: 0.8;
}

/* Dark mode support */
@media (prefers-color-scheme: dark)
{
	.loading-overlay
	{
		background: rgba(0, 0, 0, 0.3);
	}

	.progress-bar
	{
		background: rgba(255, 255, 255, 0.1);
	}
}
</style>
