<template>
  <div class="flex flex-col items-center min-h-screen p-6 w-full">
    <h1 class="text-3xl font-bold mb-6">
      CEMM - ChillEcke Modpack Manager
    </h1>
    <div class="w-full max-w-3xl">
      <component :is="panelComponent" />
    </div>
    <NuxtLink
      to="/settings"
      class="link mt-8"
    >Settings</NuxtLink>
  </div>
</template>

<script setup lang="ts">
import AdminPanel from '~/components/AdminPanel.vue'
import UserPanel from '~/components/UserPanel.vue'
import { useAppStore } from '~/stores/app'

const appStore = useAppStore()
const panelComponent = computed(() =>
	appStore.mode === 'admin' ? AdminPanel : UserPanel
)
</script>
