<template>
  <!--
    The frame both counters share. One screen that grows: a fixed heading, a
    context bar carrying the only per-use input, a scrolling work area, and a
    pinned action bar.

    There is deliberately no step rail. The previous build paginated each flow
    into steps, but most of those steps were configuration that never changes
    between uses — a settings page wearing a step number. Anything stable now
    lives in first-run setup or Settings, and what remains is a single surface.
  -->
  <div class="flex min-h-0 flex-1 flex-col">
    <div class="shrink-0 px-6 pt-5">
      <h1 class="text-xl font-bold tracking-tight">
        {{ heading }}
      </h1>
      <p
        v-if="$slots.lede"
        class="mt-1 max-w-[72ch] text-[0.8125rem] leading-relaxed text-base-content/60"
      >
        <slot name="lede" />
      </p>
    </div>

    <!-- `fillContent` hands the height to the content instead of scrolling the
         page. Used where a single list IS the screen, so a 500-row manifest
         fills the window and scrolls internally rather than sitting in a
         360px letterbox with dead space beneath it. -->
    <!-- `relative` for the same reason the pack grid needs it: a static scroll
         container's overflow is still counted by the root scroller, which puts
         a phantom scrollbar on the window. -->
    <div
      class="relative min-h-0 flex-1 px-6 pt-3.5 pb-5"
      :class="fillContent ? 'flex flex-col overflow-hidden' : 'overflow-y-auto'"
    >
      <!-- The context bar scrolls WITH the content rather than being pinned.
           Pinning it cost ~150px permanently and, worse, the diff scrolled
           underneath it — so the deletions panel, the most safety-critical
           thing on the screen, was partly hidden behind an input the user had
           already finished with. What must stay visible at commit time lives in
           the action bar instead. -->
      <div
        v-if="$slots.context"
        class="mb-3.5 shrink-0"
      >
        <slot name="context" />
      </div>

      <slot />
    </div>

    <div
      v-if="$slots.actions"
      class="flex shrink-0 flex-wrap items-center gap-3 border-t border-base-300 bg-base-200 px-6 py-3"
    >
      <slot name="actions" />
    </div>
  </div>
</template>

<script setup lang="ts">
withDefaults(defineProps<{ heading: string, fillContent?: boolean }>(), {
	fillContent: false
})
</script>
