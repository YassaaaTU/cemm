<template>
  <div class="p-6">
    <button
      class="btn btn-ghost mb-4"
      @click="goBack"
    >
      ← Back
    </button>
    <h1 class="text-2xl font-bold mb-6">
      Settings
    </h1>

    <!-- GitHub Settings -->
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

    <!-- Theme Settings -->
    <div class="card bg-base-200 mb-6">
      <div class="card-body">
        <h2 class="card-title">
          Appearance
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          Customize the look and feel of the application.
        </p>
        <div class="flex items-center gap-4">
          <span class="text-sm">Theme:</span>
          <div class="join">
            <button
              class="btn join-item"
              :class="{ 'btn-primary': currentTheme === 'nord' }"
              @click="setTheme('nord')"
            >
              <Icon
                name="mdi:white-balance-sunny"
                size="1.2rem"
                class="mr-2"
              />
              Light
            </button>
            <button
              class="btn join-item"
              :class="{ 'btn-primary': currentTheme === 'dracula' }"
              @click="setTheme('dracula')"
            >
              <Icon
                name="mdi:moon-waning-crescent"
                size="1.2rem"
                class="mr-2"
              />
              Dark
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Update Settings -->
    <div class="card bg-base-200 mb-6">
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
        </div>

        <!-- Update Status -->
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

    <!-- About CEMM -->
    <div class="card bg-base-200">
      <div class="card-body">
        <h2 class="card-title">
          About CEMM
        </h2>
        <p class="text-sm text-base-content/70 mb-4">
          Custom Edition Modpack Manager - A tool for managing and distributing Minecraft modpack updates.
        </p>
        <div class="space-y-2">
          <div class="flex items-center gap-2">
            <Icon
              name="mdi:tag"
              size="1.2rem"
            />
            <span class="text-sm">Version: {{ appVersion }}</span>
          </div>
          <div class="flex items-center gap-2">
            <Icon
              name="mdi:github"
              size="1.2rem"
            />
            <a
              href="https://github.com/YassaaaTU/cemm"
              target="_blank"
              rel="noopener noreferrer"
              class="link link-primary text-sm"
            >
              GitHub Repository
            </a>
          </div>
          <div class="flex items-center gap-2">
            <Icon
              name="mdi:file-document"
              size="1.2rem"
            />
            <a
              href="https://github.com/YassaaaTU/cemm/blob/main/LICENSE"
              target="_blank"
              rel="noopener noreferrer"
              class="link link-primary text-sm"
            >
              MIT License
            </a>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useThemeStore } from '~/stores/theme'

const router = useRouter()
const updater = useUpdater()
const themeStore = useThemeStore()

const lastUpdateCheck = ref<string>('')
const updateStatus = ref<{
	type: 'success' | 'error' | 'info'
	message: string
} | null>(null)

// Theme computed properties
const currentTheme = computed(() => themeStore.current)
const setTheme = (theme: 'nord' | 'dracula') =>
{
	themeStore.setTheme(theme)
}

// App version (from package.json or Tauri)
const appVersion = ref('1.0.0')

// Get version from Tauri if available
onMounted(async () =>
{
	if (import.meta.client)
	{
		try
		{
			const { getVersion } = await import('@tauri-apps/api/app')
			const version = await getVersion()
			appVersion.value = version
		}
		catch
		{
			// Fallback to package.json version - not running in Tauri
		}
	}
})

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

// Use the default layout (with sidebar)
definePageMeta({
	layout: 'default'
})
</script>
