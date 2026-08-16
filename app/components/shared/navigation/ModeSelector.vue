<template>
  <div class="join">
    <button
      v-for="opt in modeOptions"
      :key="opt.value"
      class="btn join-item btn-sm"
      :class="{ 'btn-primary': appStore.mode === opt.value }"
      :aria-pressed="appStore.mode === opt.value"
      @click="appStore.mode = opt.value"
    >
      <Icon
        v-if="opt.icon"
        :name="opt.icon"
        size="1.1rem"
      />
      {{ opt.label }}
    </button>
  </div>
</template>

<script setup lang="ts">
import { useAppStore } from '~/stores/app'

const appStore = useAppStore()

const modeOptions = [
	{ value: 'admin', label: 'Admin', icon: 'mdi:shield-crown' },
	{ value: 'user', label: 'User', icon: 'mdi:account' }
] as const
</script>
