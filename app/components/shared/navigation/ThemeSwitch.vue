<template>
  <div>
    <button
      type="button"
      class="btn btn-ghost btn-sm h-7 w-9 px-0 text-base-content/70 hover:bg-base-300 hover:text-base-content"
      popovertarget="cemm-theme-menu"
      style="anchor-name:--cemm-theme-menu"
      :aria-label="`Appearance: ${activeLabel}`"
    >
      <Icon
        :name="activeIcon"
        size="1rem"
        aria-hidden="true"
      />
    </button>

    <ul
      id="cemm-theme-menu"
      class="dropdown dropdown-end menu z-10 w-48 border border-base-300 bg-base-100 p-1 text-base-content shadow-lg"
      popover
      style="position-anchor:--cemm-theme-menu"
    >
      <li class="menu-title px-2 pb-1 pt-1.5 text-[0.625rem] font-bold uppercase tracking-[0.16em] text-base-content/50">
        Appearance
      </li>
      <li
        v-for="option in options"
        :key="option.value"
      >
        <button
          type="button"
          class="flex cursor-pointer items-center gap-2.5 text-sm"
          :class="{ 'menu-active': themeStore.preference === option.value }"
          @click="choose(option.value)"
        >
          <Icon
            :name="option.icon"
            size="1rem"
            aria-hidden="true"
          />
          <span class="grow text-left">{{ option.label }}</span>
          <!-- Selection is stated in text for a screen reader, not just by the
               highlighted row. -->
          <Icon
            v-if="themeStore.preference === option.value"
            name="mdi:check"
            size="0.875rem"
            aria-hidden="true"
          />
          <span
            v-if="themeStore.preference === option.value"
            class="sr-only"
          >
            (selected)
          </span>
        </button>
      </li>
    </ul>
  </div>
</template>

<script setup lang="ts">
import type { ThemePreference } from '~/stores/theme'

const themeStore = useThemeStore()

const options: Array<{ value: ThemePreference, label: string, icon: string }> = [
	{ value: 'system', label: 'Match system', icon: 'mdi:monitor' },
	{ value: 'cemm-light', label: 'Light', icon: 'mdi:white-balance-sunny' },
	{ value: 'cemm-dark', label: 'Dark', icon: 'mdi:weather-night' }
]

const activeLabel = computed(
	() => options.find((option) => option.value === themeStore.preference)?.label ?? 'Match system'
)

/**
 * When following the system the icon shows the monitor, not the resolved theme:
 * the control communicates the *preference*, and showing a sun while set to
 * "match system" would imply the user had chosen light.
 */
const activeIcon = computed(
	() => options.find((option) => option.value === themeStore.preference)?.icon ?? 'mdi:monitor'
)

const choose = (next: ThemePreference) =>
{
	themeStore.setPreference(next)
	if (import.meta.client)
	{
		document.getElementById('cemm-theme-menu')?.hidePopover()
	}
}
</script>
