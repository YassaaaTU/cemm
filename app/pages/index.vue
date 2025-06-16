<template>
  <div class="container mx-auto px-4 py-6 min-h-screen flex flex-col">
    <!-- Main content area with responsive constraints -->
    <main
      id="main-content"
      class="flex-1 w-full max-w-5xl mx-auto"
      tabindex="-1"
    >
      <component :is="panelComponent" />
    </main>

    <!-- Footer with responsive spacing -->
    <footer class="mt-8 pt-6 border-t border-base-300 text-center text-sm opacity-70">
      <p class="mb-2">
        CEMM v1.0 - ChillEcke Modpack Manager
      </p>
      <p class="text-xs">
        Built with Nuxt 3, Tauri, and Rust
      </p>
    </footer>
  </div>
</template>

<script setup lang="ts">
import AdminPanel from '~/components/AdminPanel.vue'
import UserPanel from '~/components/UserPanel.vue'
import { useAppStore } from '~/stores/app'

const appStore = useAppStore()

// Dynamic component based on current mode
const panelComponent = computed(() =>
	appStore.mode === 'admin' ? AdminPanel : UserPanel
)
</script>
