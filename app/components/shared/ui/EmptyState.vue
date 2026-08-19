<template>
  <!-- The resting state of a workspace before there is anything to act on.
       Carries the next action rather than only an apology for being empty. -->
  <div class="flex min-h-64 flex-col items-center justify-center gap-2.5 px-6 text-center">
    <span class="grid size-12 place-items-center rounded-box bg-base-200 text-base-content/40">
      <Icon
        :name="icon"
        size="1.375rem"
        aria-hidden="true"
      />
    </span>
    <h2 class="text-[0.9375rem] font-semibold text-base-content/75">
      {{ title }}
    </h2>
    <p class="max-w-[46ch] text-[0.8125rem] leading-relaxed text-base-content/55">
      <slot />
    </p>
    <!-- Wraps rather than stacks: a resting state can carry a primary action and
         a way round it — "browse your packs" beside "choose a file instead" —
         and two bare buttons in a plain div sit flush against each other. -->
    <div
      v-if="$slots.action"
      class="mt-1.5 flex flex-wrap items-center justify-center gap-2"
    >
      <slot name="action" />
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{ icon: string, title: string }>()
</script>
