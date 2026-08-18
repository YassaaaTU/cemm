<template>
  <!-- One setting per row: name and explanation on the left, control on the
       right. With this few settings it reads faster than stacked fieldsets, and
       every control lands on the same vertical axis. -->
  <div class="flex flex-wrap items-center justify-between gap-x-6 gap-y-3 px-4 py-3.5">
    <div class="min-w-0 flex-1 basis-64 max-w-[54ch]">
      <component
        :is="labelFor.length > 0 ? 'label' : 'p'"
        :for="labelFor.length > 0 ? labelFor : undefined"
        class="block text-sm font-medium"
      >
        {{ label }}
      </component>
      <p
        v-if="$slots.description"
        class="mt-0.5 text-xs leading-relaxed text-base-content/55"
      >
        <slot name="description" />
      </p>
    </div>

    <div class="flex shrink-0 items-center gap-2">
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
withDefaults(defineProps<{ label: string, labelFor?: string }>(), {
	labelFor: ''
})
</script>
