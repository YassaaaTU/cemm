<template>
  <!-- The narrow icon rail both CurseForge and Modrinth use. Destinations are
       icon-only with tooltips and accessible names, so the rail costs 54px
       instead of the 200px the old fixed sidebar took from the content. -->
  <nav
    class="flex w-[54px] shrink-0 flex-col items-center gap-1.5 border-r border-base-300 bg-base-200 py-2"
    aria-label="Main"
  >
    <NuxtLink
      v-for="item in destinations"
      :key="item.to"
      :to="item.to"
      class="tooltip tooltip-right relative grid size-[38px] place-items-center rounded-lg transition-colors duration-150 ease-[var(--ease-standard)] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
      :class="isActive(item)
        ? 'bg-primary/15 text-primary'
        : 'text-base-content/50 hover:bg-base-300 hover:text-base-content'"
      :data-tip="item.label"
      :aria-label="item.label"
      :aria-current="isActive(item) ? 'page' : undefined"
      @click="item.mode !== undefined && selectMode(item.mode)"
    >
      <!-- The active marker is a shape as well as a colour, so the current
           destination is not signalled by hue alone. -->
      <span
        v-if="isActive(item)"
        class="absolute -left-2 top-2.5 bottom-2.5 w-[3px] rounded-r bg-primary"
        aria-hidden="true"
      />
      <Icon
        :name="item.icon"
        size="1.1875rem"
        aria-hidden="true"
      />
    </NuxtLink>

    <div class="flex-1" />

    <NuxtLink
      to="/settings"
      class="tooltip tooltip-right relative grid size-[38px] place-items-center rounded-lg transition-colors duration-150 ease-[var(--ease-standard)] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
      :class="route.path === '/settings'
        ? 'bg-primary/15 text-primary'
        : 'text-base-content/50 hover:bg-base-300 hover:text-base-content'"
      data-tip="Settings"
      aria-label="Settings"
      :aria-current="route.path === '/settings' ? 'page' : undefined"
    >
      <span
        v-if="route.path === '/settings'"
        class="absolute -left-2 top-2.5 bottom-2.5 w-[3px] rounded-r bg-primary"
        aria-hidden="true"
      />
      <Icon
        name="mdi:cog-outline"
        size="1.1875rem"
        aria-hidden="true"
      />
    </NuxtLink>
  </nav>
</template>

<script setup lang="ts">
import type { AppMode } from '~/stores/app'

const route = useRoute()
const appStore = useAppStore()
const manifestStore = useManifestStore()

interface Destination
{
	label: string
	to: string
	icon: string
	mode?: AppMode
}

const destinations: Destination[] = [
	{ label: 'Install update', to: '/dashboard', icon: 'mdi:tray-arrow-down', mode: 'user' },
	{ label: 'Publish update', to: '/dashboard', icon: 'mdi:tray-arrow-up', mode: 'admin' }
]

/**
 * Both counters live on /dashboard, so the active rail item is decided by the
 * current mode rather than by the route alone.
 */
const isActive = (item: Destination) =>
	route.path === '/dashboard' && item.mode === appStore.mode

const selectMode = (next: AppMode) =>
{
	if (appStore.mode === next) return
	manifestStore.clearManifest()
	appStore.setMode(next)
}
</script>
