<template>
  <div>
    <!-- SPA Navigation: Conditionally render page components based on navigation store -->
    <landing-page v-if="navigation.isLanding" />
    <NuxtLayout
      v-else
      name="default"
    >
      <dashboard-page v-if="navigation.isDashboard" />
      <settings-page v-if="navigation.isSettings" />
    </NuxtLayout>
  </div>
</template>

<script setup lang="ts">
import DashboardPage from './components/pages/DashboardPage.vue'
import LandingPage from './components/pages/LandingPage.vue'
import SettingsPage from './components/pages/SettingsPage.vue'
import { useUpdater } from './composables/useUpdater'

const navigation = useNavigationStore()

// Hooks for app startup
onMounted(() =>
{
	const { checkForUpdatesOnStartup } = useUpdater()

	// Check for updates automatically on startup (desktop only)
	if (import.meta.client)
	{
		checkForUpdatesOnStartup()
	}
})
</script>
