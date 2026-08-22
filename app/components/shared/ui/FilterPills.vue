<template>
  <!-- One pill row. The count sits inside the pill rather than beside the list,
       because the question a filter answers is "how many would this leave?" and
       a player should not have to click to find out. -->
  <div
    class="flex flex-wrap"
    :class="size === 'md' ? 'gap-2' : 'gap-1.5'"
    role="group"
    :aria-label="label"
  >
    <button
      v-for="option in options"
      :key="option.value"
      type="button"
      class="cursor-pointer rounded-full border font-medium transition-colors duration-150 ease-(--ease-standard)"
      :class="[
        size === 'md' ? 'px-3 py-1 text-[0.8125rem]' : 'px-2.5 py-0.5 text-xs',
        modelValue === option.value
          ? 'border-primary bg-primary/15 text-primary'
          : `border-base-300 ${size === 'md' ? 'bg-base-200' : 'bg-base-100'} text-base-content/60 hover:text-base-content`,
      ]"
      :aria-pressed="modelValue === option.value"
      @click="emit('update:modelValue', option.value)"
    >
      {{ option.label }}
      <span
        v-if="option.count !== undefined"
        class="ml-1 font-mono tabular-nums opacity-60"
      >{{ option.count }}</span>
    </button>
  </div>
</template>

<script setup lang="ts" generic="T extends string">
export interface FilterOption<Value extends string>
{
	value: Value
	label: string
	/** Omit for a pill whose count would say nothing, never to hide a zero. */
	count?: number
}

/**
 * The app's one filter-pill row: the admin's category panes, and both
 * dimensions of the player's diff. They were three copies of the same markup,
 * which is how the diff's pills ended up a size apart from the admin's for no
 * reason anyone recorded.
 *
 * `md` is for pills sitting on the page ground, `sm` for pills inside a list
 * header — the idle fill follows from that, always one step off the surface
 * behind it.
 */
withDefaults(
	defineProps<{
		options: ReadonlyArray<FilterOption<T>>
		modelValue: T
		/** Names the group for screen readers. Two pill rows can both say "All". */
		label: string
		size?: 'sm' | 'md'
	}>(),
	{ size: 'sm' }
)

const emit = defineEmits<{
	'update:modelValue': [value: T]
}>()
</script>
