<template>
  <div>
    <NuxtLayout>
      <NuxtPage />
    </NuxtLayout>
  </div>
</template>

<script setup lang="ts">
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
