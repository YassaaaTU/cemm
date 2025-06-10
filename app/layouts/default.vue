<template>
  <div class="min-h-screen flex flex-col bg-base-100 text-base-content">
    <header class="navbar bg-base-200 shadow mb-4">
      <div class="navbar-start">
        <span class="font-bold text-lg ml-4">CEMM</span>
      </div>
      <div class="navbar-center">
        <mode-selector />
      </div>
      <div class="navbar-end">
        <NuxtLink
          to="/settings"
          class="btn btn-ghost mr-4"
        >Settings</NuxtLink>
      </div>
    </header>
    <main class="flex-1">
      <slot />
    </main>
  </div>
</template>

<script setup lang="ts">
import ModeSelector from '~/components/ModeSelector.vue'
</script>
