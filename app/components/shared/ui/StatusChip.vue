<template>
  <!-- Status is carried by the word first and the colour second, so the diff
       still reads correctly in greyscale and to anyone who cannot separate the
       hues. Colour never operates alone here. -->
  <span
    class="inline-flex shrink-0 items-center rounded-md border px-2 py-px text-[0.6875rem] font-semibold leading-[1.5]"
    :class="toneClass"
  >
    <slot>{{ label }}</slot>
  </span>
</template>

<script setup lang="ts">
export type StatusTone = 'new' | 'updated' | 'removed' | 'unchanged' | 'excluded' | 'shipping'

const props = withDefaults(
	defineProps<{ tone: StatusTone, label?: string }>(),
	{ label: '' }
)

const toneClass = computed(() =>
{
	const tones: Record<StatusTone, string> = {
		new: 'border-success/45 bg-success/10 text-success',
		updated: 'border-info/45 bg-info/10 text-info',
		removed: 'border-error/45 bg-error/10 text-error',
		unchanged: 'border-base-content/20 text-base-content/55',
		excluded: 'border-error/45 bg-error/10 text-error',
		shipping: 'border-base-content/20 text-base-content/55'
	}
	return tones[props.tone]
})
</script>
