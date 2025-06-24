<!-- Console logging for debugging update checks -->
<template>
  <div class="p-6">
    <button
      class="btn btn-ghost mb-4"
      @click="goBack"
    >
      ← Back
    </button>    <h1 class="text-2xl font-bold mb-6">
      Settings
    </h1>    <!-- GitHub Settings -->
    <div class="card bg-base-200 mb-6">
      <div class="card-body">
        <h2 class="card-title">
          Modpack Updates Repository
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          Configure the GitHub repository where your modpack updates are stored (e.g., YassaaaTU/cemm-updates).
        </p>
        <GitHubSettings />
      </div>
    </div>

    <!-- Update Settings -->
    <div class="card bg-base-200">
      <div class="card-body">
        <h2 class="card-title">
          Application Updates
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          Check for the latest version of CEMM from the app repository (YassaaaTU/cemm).
        </p>

        <div class="flex items-center gap-4">
          <button
            class="btn btn-primary"
            :disabled="updater.isChecking.value"
            @click="handleCheckForUpdates"
          >
            <Icon
              v-if="updater.isChecking.value"
              name="line-md:loading-loop"
              size="1.4em"
            />
            <Icon
              v-else
              name="mdi:update"
              size="1.4em"
            />
            {{ updater.isChecking.value ? 'Checking...' : 'Check for Updates' }}
          </button>

          <div
            v-if="lastUpdateCheck"
            class="text-sm text-base-content/60"
          >
            Last checked: {{ lastUpdateCheck }}
          </div>
        </div>        <!-- Update Status -->
        <div
          v-if="updateStatus"
          class="mt-4"
        >
          <div
            class="alert"
            :class="updateStatus.type === 'success' ? 'alert-success' : updateStatus.type === 'error' ? 'alert-error' : 'alert-info'"
          >
            <Icon
              :name="updateStatus.type === 'success' ? 'mdi:check-circle' : updateStatus.type === 'error' ? 'mdi:alert-circle' : 'mdi:information'"
            />
            <span>{{ updateStatus.message }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
const router = useRouter()
const updater = useUpdater()
// const { $logger } = useNuxtApp()

const lastUpdateCheck = ref<string>('')
const updateStatus = ref<{
	type: 'success' | 'error' | 'info'
	message: string
} | null>(null)

const goBack = () =>
{
	router.back()
}

const handleCheckForUpdates = async () =>
{
	updateStatus.value = null

	try
	{
		const result = await updater.checkForUpdates()
		lastUpdateCheck.value = new Date().toLocaleString()
		// Debug logging for update check
		console.log('Update check result:', {
			available: result.available,
			currentVersion: result.current_version,
			latestVersion: result.latest_version,
			downloadUrl: result.download_url,
			assetName: result.asset_name
		})

		// Debug: Check if dialog should be visible
		console.info('UpdateDialog state after check:', {
			isUpdateDialogVisible: updater.isUpdateDialogVisible.value,
			updateInfo: updater.updateInfo.value
		})

		if (result.available)
		{
			console.info('✅ Update available!', `${result.current_version} → ${result.latest_version}`)

			updateStatus.value = {
				type: 'info',
				message: `Update available: v${result.latest_version}. The update dialog will appear automatically.`
			}
		}
		else
		{
			console.info('✅ Already up to date!', `Current version: ${result.current_version}`)

			updateStatus.value = {
				type: 'success',
				message: `You're running the latest version (v${result.current_version}).`
			}
		}
	}
	catch (error)
	{
		console.error('❌ Update check failed:', error)

		updateStatus.value = {
			type: 'error',
			message: `Failed to check for updates: ${error}`
		}
	}
}
</script>
