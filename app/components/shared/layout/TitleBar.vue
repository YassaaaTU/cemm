<template>
  <header
    class="flex h-9 shrink-0 [user-select:none] items-center gap-2.5 border-b border-base-300 bg-base-200 pl-3 [-webkit-app-region:drag]"
    @dblclick="onTitleBarDoubleClick"
  >
    <div class="flex items-center gap-2">
      <BrandMark class="size-[17px] shrink-0 text-primary" />
      <span class="text-[0.8125rem] font-bold tracking-[0.04em]">CEMM</span>
    </div>

    <!-- Mode switching lives in the icon rail only. This bar used to carry a
         second segmented control doing exactly the same job 40px away; one
         destination should have one control. -->

    <!-- The empty middle is the drag handle for moving the window. -->
    <div class="min-w-4 flex-1" />

    <div
      class="flex items-center gap-1 self-stretch [-webkit-app-region:no-drag]"
      data-no-drag
    >
      <ThemeSwitch />

      <!-- Hidden outside Tauri so `nuxt dev` in a browser tab does not show
           three controls that cannot do anything. -->
      <div
        v-if="isDesktop"
        class="flex items-stretch self-stretch"
      >
        <button
          type="button"
          class="flex w-10 items-center justify-center text-base-content/55 transition-colors duration-100 hover:bg-base-300 hover:text-base-content focus-visible:outline-2 focus-visible:outline-offset-[-3px] focus-visible:outline-primary"
          aria-label="Minimise window"
          @click="minimize"
        >
          <Icon
            name="mdi:window-minimize"
            size="0.9375rem"
            aria-hidden="true"
          />
        </button>
        <button
          type="button"
          class="flex w-10 items-center justify-center text-base-content/55 transition-colors duration-100 hover:bg-base-300 hover:text-base-content focus-visible:outline-2 focus-visible:outline-offset-[-3px] focus-visible:outline-primary"
          :aria-label="isMaximized ? 'Restore window' : 'Maximise window'"
          @click="toggleMaximize"
        >
          <Icon
            :name="isMaximized ? 'mdi:window-restore' : 'mdi:window-maximize'"
            size="0.9375rem"
            aria-hidden="true"
          />
        </button>
        <!-- Close is the only control with a destructive hover, matching both
             Windows and the common Linux desktops. -->
        <button
          type="button"
          class="flex w-10 items-center justify-center text-base-content/55 transition-colors duration-100 hover:bg-error hover:text-error-content focus-visible:outline-2 focus-visible:outline-offset-[-3px] focus-visible:outline-primary"
          aria-label="Close window"
          @click="close"
        >
          <Icon
            name="mdi:window-close"
            size="0.9375rem"
            aria-hidden="true"
          />
        </button>
      </div>
    </div>
  </header>
</template>

<script setup lang="ts">
const {
	isDesktop,
	isMaximized,
	minimize,
	toggleMaximize,
	close,
	onTitleBarDoubleClick
} = useWindowControls()
</script>
