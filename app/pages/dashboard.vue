<template>
  <div class="flex min-h-0 flex-1 flex-col">
    <component :is="panelComponent" />
  </div>
</template>

<script setup lang="ts">
// The layout owns the <main> landmark, page width and padding, so this page is
// a straight switch between the two counters and nothing else. The wrapper div
// exists because Nuxt page transitions require a single static root node.
import AdminPanel from '~/components/features/admin/AdminPanel.vue'
import UserPanel from '~/components/features/user/UserPanel.vue'
import { useAppStore } from '~/stores/app'

const appStore = useAppStore()

const panelComponent = computed(() =>
	appStore.mode === 'admin' ? AdminPanel : UserPanel
)

definePageMeta({
	layout: 'default'
})
</script>
