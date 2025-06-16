<template>
  <div class="container mx-auto px-4 py-6 flex flex-col flex-1 min-h-0">
    <!-- Main content area with responsive constraints -->
    <main
      id="main-content"
      class="flex-1 w-full max-w-5xl mx-auto"
      tabindex="-1"
    >
      <component :is="panelComponent" />
    </main>
  </div>
</template>

<script setup lang="ts">
import AdminPanel from '~/components/AdminPanel.vue'
import UserPanel from '~/components/UserPanel.vue'

const appStore = useAppStore()

// Dynamic component based on current mode
const panelComponent = computed(() =>
	appStore.mode === 'admin' ? AdminPanel : UserPanel
)
</script>
