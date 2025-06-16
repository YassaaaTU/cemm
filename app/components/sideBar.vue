<template>
  <aside
    class="fixed top-0 left-0 h-screen z-30 w-56 bg-base-200 border-r border-base-300 flex flex-col pt-4"
    role="navigation"
    aria-label="Main sidebar navigation"
  >
    <!-- Mode selector -->
    <div class="w-full flex flex-col items-center sm:items-stretch mb-2">
      <mode-selector />
    </div>

    <div class="divider m-0" />

    <!-- Navigation links -->
    <nav class="flex flex-col w-full items-center sm:items-stretch mt-2">
      <button
        class="btn rounded-none m-0 btn-lg justify-start"
        aria-label="Home"
        :class="{ 'btn-primary': navigation.isLanding }"
        @click="navigation.navigateToLanding()"
      >
        <Icon
          name="mdi:home"
          size="1.2rem"
          class="mr-0 sm:mr-2"
        />
        <span class="hidden sm:inline">Home</span>
      </button>
      <button
        class="btn rounded-none m-0 btn-lg justify-start"
        aria-label="Dashboard"
        :class="{ 'btn-primary': navigation.isDashboard }"
        @click="navigation.navigateToDashboard()"
      >
        <Icon
          name="mdi:view-dashboard"
          size="1.2rem"
          class="mr-0 sm:mr-2"
        />
        <span class="hidden sm:inline">Dashboard</span>
      </button>
      <button
        class="btn rounded-none m-0 btn-lg justify-start"
        aria-label="Settings"
        :class="{ 'btn-primary': navigation.isSettings }"
        @click="navigation.navigateToSettings()"
      >
        <Icon
          name="mdi:cog"
          size="1.2rem"
          class="mr-0 sm:mr-2"
        />
        <span class="hidden sm:inline">Settings</span>
      </button>
    </nav>

    <div class="w-full mt-auto flex flex-col">
      <!-- divider -->
      <div class="divider m-0" />

      <!-- Theme toggle -->
      <button
        class="btn btn-ghost h-full m-0 btn-lg space-x-2 py-2 rounded-none"
        aria-label="Toggle theme"
        @click="toggleTheme"
      >
        <Icon
          name="mdi:theme-light-dark"
          size="1.2rem"
        />
        <span class="hidden sm:inline">{{ isDark ? 'Light Mode' : 'Dark Mode' }}</span>
      </button>
    </div>
  </aside>
</template>

<script lang="ts" setup>
import { useThemeStore } from '~/stores/theme'

const themeStore = useThemeStore()
const navigation = useNavigationStore()

// Theme computed properties
const isDark = computed(() => themeStore.isdark)

// Theme toggle function
const toggleTheme = () =>
{
	themeStore.toggleTheme()
}

// Apply theme to document
watchEffect(() =>
{
	if (import.meta.client)
	{
		document.documentElement.setAttribute('data-theme', themeStore.current)
	}
})
</script>
