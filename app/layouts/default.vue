<template>
  <div class="flex h-dvh flex-col overflow-hidden bg-base-100 font-sans text-base-content antialiased">
    <a
      href="#main-content"
      class="sr-only left-2 top-2 z-50 rounded-lg border border-primary bg-base-100 px-3 py-2 text-sm font-semibold text-base-content focus:not-sr-only focus:absolute"
    >
      Skip to main content
    </a>

    <TitleBar />

    <div class="flex min-h-0 flex-1">
      <IconRail />

      <!-- The single <main> landmark for every page using this layout. Pages
           must not add their own <main> or role="main", or screen-reader
           landmark navigation sees nested "main" regions (F-P2-8). The id and
           tabindex give the skip link a target shared across pages. -->
      <main
        id="main-content"
        tabindex="-1"
        class="flex min-h-0 min-w-0 flex-1 flex-col"
      >
        <slot />
      </main>
    </div>

    <UpdateDialog />

    <!-- Toasts sit bottom-right, above the pinned action bar rather than over
         the diff, and follow the resolved theme so they never appear as a light
         card on a dark app. -->
    <toaster
      :theme="themeStore.isDark ? 'dark' : 'light'"
      position="bottom-right"
      :offset="'72px'"
      close-button
      rich-colors
      :visible-toasts="4"
      :toast-options="toastOptions"
    />
  </div>
</template>

<script setup lang="ts">
import { Toaster } from 'vue-sonner'

const themeStore = useThemeStore()

/**
 * Sonner's own surface is replaced with the app's, so a toast reads as part of
 * CEMM rather than as a third-party widget. Only the container is restyled —
 * its positioning, stacking and swipe behaviour are left alone.
 */
const toastOptions = {
	classes: {
		// Only radius and type are ours. Sonner owns the surface: richColors
		// already derives semantic backgrounds per type and follows the theme
		// prop, so overriding bg here would fight it and flatten success/error
		// back into identical grey cards.
		toast: 'rounded-box font-sans shadow-lg',
		title: 'text-sm font-semibold',
		description: 'text-xs opacity-80',
		actionButton: 'btn btn-xs btn-primary',
		cancelButton: 'btn btn-xs'
	}
}
</script>
