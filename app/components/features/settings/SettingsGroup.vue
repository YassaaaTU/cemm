<template>
  <section class="mb-6">
    <!-- Plain sentence-case heading, not an uppercase tracked eyebrow. The tabbed
         build stacked "PREFERENCES", "REPOSITORY" and "ACCESS TOKEN" in that
         style, which is a templated rhythm rather than real hierarchy. -->
    <h2 class="mb-2 px-1 text-sm font-semibold text-base-content/80">
      {{ title }}
    </h2>

    <!-- `as` exists so a group can be a <form>. The dividers come from divide-y
         on this element, so its rows have to be its direct children. -->
    <component
      :is="as"
      class="divide-y divide-base-300 overflow-hidden rounded-box border border-base-300 bg-base-200"
    >
      <slot />
    </component>

    <p
      v-if="$slots.footnote"
      class="mt-2 px-1 text-xs/relaxed  text-base-content/50"
    >
      <slot name="footnote" />
    </p>
  </section>
</template>

<script setup lang="ts">
withDefaults(defineProps<{ title: string, as?: string }>(), {
	as: 'div'
})
</script>
