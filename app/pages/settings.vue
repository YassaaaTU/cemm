<template>
  <div class="min-h-0 flex-1 overflow-y-auto">
    <!--
      One scrolling page, no tabs. The tabbed build put four settings behind four
      tabs, so three quarters of them were always hidden and every pane except
      Repository was mostly empty. Tabs earn their navigation cost when a pane
      holds more than you can take in at once; here they made you click to find
      out there was almost nothing behind them.
    -->
    <div class="mx-auto w-full max-w-3xl px-6 py-6">
      <h1 class="mb-5 text-2xl font-bold tracking-tight">
        Settings
      </h1>

      <GitHubSettings />

      <SettingsGroup title="Appearance">
        <SettingsRow label="Theme">
          <template #description>
            Match system follows your desktop the moment you change it there.<template v-if="themeStore.preference === 'system'">
              Currently showing {{ themeStore.isDark ? 'dark' : 'light' }}.
            </template>
          </template>

          <div class="join">
            <button
              v-for="option in themeOptions"
              :key="option.value"
              type="button"
              class="btn join-item btn-sm gap-1.5 border-base-300"
              :class="themeStore.preference === option.value ? 'btn-primary' : ''"
              :aria-pressed="themeStore.preference === option.value"
              @click="themeStore.setPreference(option.value)"
            >
              <Icon
                :name="option.icon"
                size="1rem"
                aria-hidden="true"
              />
              {{ option.label }}
            </button>
          </div>
        </SettingsRow>

        <SettingsRow
          label="Reduce animation"
          label-for="settings-motion"
        >
          <template #description>
            Turns off panel transitions and progress easing. If your system
            already asks for reduced motion, that is honoured either way.
          </template>

          <!-- The visible <label for> is emitted by SettingsRow, so the naming
               is not readable from this file alone. aria-label repeats that exact
               text to keep the association explicit here. -->
          <input
            id="settings-motion"
            type="checkbox"
            class="toggle toggle-sm"
            aria-label="Reduce animation"
            :checked="themeStore.motion === 'reduced'"
            @change="themeStore.toggleMotion()"
          />
        </SettingsRow>
      </SettingsGroup>

      <!--
        Updates was its own tab holding a single version string and a single
        button, and About was a tab of static text that is not a setting at all.
        They are the same subject, so they are now one block.
      -->
      <SettingsGroup title="About">
        <div class="flex items-start gap-4 px-4 py-4">
          <BrandMark class="mt-0.5 size-9 shrink-0 text-primary" />

          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-baseline gap-x-2.5 gap-y-1">
              <p class="text-base font-bold">
                CEMM
              </p>
              <p class="font-mono text-xs tabular-nums text-accent">
                v{{ appVersion ?? packageVersion }}
              </p>
            </div>

            <p class="mt-1.5 text-sm leading-relaxed text-base-content/65">
              ChillEcke Modpack Manager. Distributes changes to an existing
              CurseForge modpack among a group, without republishing the whole pack.
            </p>

            <p class="mt-2 flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-base-content/50">
              <span>MIT licence</span>
              <span aria-hidden="true">·</span>
              <span>YassaaaTU</span>
              <span aria-hidden="true">·</span>
              <a
                href="https://github.com/YassaaaTU/cemm"
                target="_blank"
                rel="noopener noreferrer"
                class="link link-hover inline-flex items-center gap-1 text-primary"
              >
                <Icon
                  name="mdi:github"
                  size="0.875rem"
                  aria-hidden="true"
                />
                Source on GitHub
              </a>
            </p>
          </div>
        </div>

        <div class="flex flex-wrap items-center justify-between gap-x-6 gap-y-3 px-4 py-3.5">
          <p class="text-xs text-base-content/55">
            <template v-if="lastUpdateCheck.length > 0">
              Last checked {{ lastUpdateCheck }}.
            </template>
            <template v-else>
              CEMM checks for its own updates on launch.
            </template>
          </p>

          <button
            type="button"
            class="btn btn-sm gap-1.5 border-base-300"
            :disabled="checking"
            @click="handleCheckForUpdates"
          >
            <span
              v-if="checking"
              class="loading loading-spinner loading-xs"
              aria-hidden="true"
            />
            <Icon
              v-else
              name="mdi:refresh"
              size="1rem"
              aria-hidden="true"
            />
            Check for updates
          </button>
        </div>
      </SettingsGroup>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { ThemePreference } from '~/stores/theme'
import pkg from '~~/package.json'

const updater = useUpdater()
const themeStore = useThemeStore()
const { notify } = useNotify()

const packageVersion = pkg.version
const lastUpdateCheck = ref('')
const checking = ref(false)
const appVersion = ref<string | null>(null)

const themeOptions: Array<{ value: ThemePreference, label: string, icon: string }> = [
	{ value: 'system', label: 'Match system', icon: 'mdi:monitor' },
	{ value: 'cemm-light', label: 'Light', icon: 'mdi:white-balance-sunny' },
	{ value: 'cemm-dark', label: 'Dark', icon: 'mdi:weather-night' }
]

onMounted(async () =>
{
	if (import.meta.client)
	{
		try
		{
			const { getVersion } = await import('@tauri-apps/api/app')
			appVersion.value = await getVersion()
		}
		catch
		{
			// Running outside Tauri (browser dev): package.json version stands in.
		}
	}
})

const handleCheckForUpdates = async () =>
{
	checking.value = true

	try
	{
		const result = await updater.checkForUpdates()
		lastUpdateCheck.value = new Date().toLocaleString()
		if (result !== null)
		{
			notify(`Version ${result.version} is available.`, 'info', 'The update dialog will open automatically.')
		}
		else
		{
			notify(`You are on the latest version (${appVersion.value ?? packageVersion}).`, 'success')
		}
	}
	catch (error)
	{
		notify('Could not check for updates.', 'error', error instanceof Error ? error.message : String(error))
	}
	finally
	{
		checking.value = false
	}
}

definePageMeta({ layout: 'default' })
</script>
