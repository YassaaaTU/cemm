<template>
  <!-- The destination rail both CurseForge and Modrinth use, in two widths.

       Compact (54px) is the default and the reason the rail exists: it costs
       54px instead of the 200px a fixed labelled sidebar takes from the diff.
       Expanded (200px) names each destination for anyone who would rather read
       than recognise an icon, and is remembered across launches.

       The icon box is a fixed 38px at the start of every row in BOTH states, so
       widening the rail never moves an icon — only the labels arrive beside
       them. That is what keeps the transition readable rather than a slide. -->
  <nav
    class="flex shrink-0 flex-col gap-1.5 overflow-hidden border-r border-base-300 bg-base-200 p-2 "
    :class="[
      appStore.railExpanded ? 'w-50' : 'w-13.5',
      anim('transition-[width] duration-220 ease-out-quick'),
    ]"
    aria-label="Main"
  >
    <NuxtLink
      v-for="item in destinations"
      :key="item.label"
      :to="item.to"
      class="relative flex h-9.5 w-full items-center overflow-hidden rounded-lg transition-colors duration-150 ease-(--ease-standard) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
      :class="[
        tooltipClass,
        isActive(item)
          ? 'bg-primary/15 text-primary'
          : 'text-base-content/50 hover:bg-base-300 hover:text-base-content',
      ]"
      :data-tip="item.label"
      :aria-label="item.label"
      :aria-current="isActive(item) ? 'page' : undefined"
      @click="item.mode !== undefined && selectMode(item.mode)"
    >
      <!-- The active marker is a shape as well as a colour, so the current
           destination is not signalled by hue alone. -->
      <span
        v-if="isActive(item)"
        class="absolute inset-y-2.5  -left-2 w-0.75 rounded-r bg-primary"
        aria-hidden="true"
      />
      <span class="grid size-9.5 shrink-0 place-items-center">
        <Icon
          :name="item.icon"
          size="1.1875rem"
          aria-hidden="true"
        />
      </span>
      <!-- Decorative: the link is already named by aria-label, so the visible
           text must not be announced a second time. -->
      <span
        class="min-w-0 pr-2 text-[0.8125rem] font-medium whitespace-nowrap"
        :class="labelClass"
        aria-hidden="true"
      >{{ item.label }}</span>
    </NuxtLink>

    <div class="flex-1" />

    <NuxtLink
      to="/settings"
      class="relative flex h-9.5 w-full items-center overflow-hidden rounded-lg transition-colors duration-150 ease-(--ease-standard) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
      :class="[
        tooltipClass,
        isSettings
          ? 'bg-primary/15 text-primary'
          : 'text-base-content/50 hover:bg-base-300 hover:text-base-content',
      ]"
      data-tip="Settings"
      aria-label="Settings"
      :aria-current="isSettings ? 'page' : undefined"
    >
      <span
        v-if="isSettings"
        class="absolute inset-y-2.5  -left-2 w-0.75 rounded-r bg-primary"
        aria-hidden="true"
      />
      <span class="grid size-9.5 shrink-0 place-items-center">
        <Icon
          name="mdi:cog-outline"
          size="1.1875rem"
          aria-hidden="true"
        />
      </span>
      <span
        class="min-w-0 pr-2 text-[0.8125rem] font-medium whitespace-nowrap"
        :class="labelClass"
        aria-hidden="true"
      >Settings</span>
    </NuxtLink>

    <!-- The width control sits at the foot of the rail it controls, on the same
         38px grid as the destinations so the column reads as one stack. -->
    <div class="mt-0.5 border-t border-base-300 pt-1.5">
      <button
        type="button"
        class="relative flex h-9.5 w-full cursor-pointer items-center overflow-hidden rounded-lg text-base-content/40 transition-colors duration-150 ease-(--ease-standard) hover:bg-base-300 hover:text-base-content focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
        :class="tooltipClass"
        :data-tip="appStore.railExpanded ? 'Collapse sidebar' : 'Expand sidebar'"
        :aria-label="appStore.railExpanded ? 'Collapse sidebar' : 'Expand sidebar'"
        :aria-expanded="appStore.railExpanded"
        @click="appStore.toggleRail()"
      >
        <span class="grid size-9.5 shrink-0 place-items-center">
          <Icon
            name="mdi:chevron-right"
            size="1.1875rem"
            :class="[
              appStore.railExpanded ? 'rotate-180' : 'rotate-0',
              anim('transition-transform duration-220 ease-out-quick'),
            ]"
            aria-hidden="true"
          />
        </span>
        <span
          class="min-w-0 pr-2 text-[0.8125rem] font-medium whitespace-nowrap"
          :class="labelClass"
          aria-hidden="true"
        >Collapse</span>
      </button>
    </div>
  </nav>
</template>

<script setup lang="ts">
import type { AppMode } from '~/stores/app'

const route = useRoute()
const appStore = useAppStore()
const manifestStore = useManifestStore()
const { anim } = useMotion()

interface Destination
{
	label: string
	to: string
	icon: string
	mode?: AppMode
}

/**
 * The library leads because it is where a task starts: it is the one
 * destination with no mode, and choosing an action on a card is what sets the
 * counter. The two counters follow it because they are where you *end up* —
 * still reachable directly, for anyone who already knows which side they are on
 * and has a pack loaded.
 *
 * It leads without being a landing screen. Nothing routes here automatically;
 * the job is still to ship or receive an update, and a library you must pass
 * through on every launch would be the interstitial this build already removed.
 */
const destinations: Destination[] = [
	{ label: 'Your packs', to: '/packs', icon: 'mdi:view-grid-outline' },
	{ label: 'Install update', to: '/dashboard', icon: 'mdi:tray-arrow-down', mode: 'user' },
	{ label: 'Publish update', to: '/dashboard', icon: 'mdi:tray-arrow-up', mode: 'admin' }
]

const isSettings = computed(() => route.path === '/settings')

/**
 * Tooltips are the compact rail's only way to name a destination, so they are
 * dropped the moment the labels are on screen — otherwise every row would state
 * its name twice, once permanently and once on hover.
 */
const tooltipClass = computed(() =>
	appStore.railExpanded ? '' : 'tooltip tooltip-right'
)

/**
 * Labels fade in slightly AFTER the rail has started widening and leave
 * immediately when it closes. Waiting on the way in stops text appearing in a
 * gap too narrow to hold it; leaving at once stops it being clipped mid-word.
 */
const labelClass = computed(() =>
{
	if (appStore.railExpanded)
	{
		return [
			'translate-x-0 opacity-100',
			anim('transition delay-90 duration-150 ease-(--ease-standard)')
		]
	}
	return [
		'-translate-x-1 opacity-0',
		anim('transition duration-100 ease-(--ease-standard)')
	]
})

/**
 * Both counters live on /dashboard, so their active state is decided by the
 * current mode rather than by the route alone. Destinations with no mode — the
 * pack library — are ordinary routes and match on the path.
 */
const isActive = (item: Destination) =>
	item.mode === undefined
		? route.path === item.to
		: route.path === '/dashboard' && item.mode === appStore.mode

const selectMode = (next: AppMode) =>
{
	if (appStore.mode === next) return
	manifestStore.clearManifest()
	appStore.setMode(next)
}
</script>
