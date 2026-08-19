<template>
  <div class="min-h-0 flex-1 overflow-y-auto">
    <!--
      One scrolling page, no tabs. The tabbed build put four settings behind four
      tabs, so three quarters of them were always hidden and every pane except
      Repository was mostly empty. Tabs earn their navigation cost when a pane
      holds more than you can take in at once; here they made you click to find
      out there was almost nothing behind them.
    -->
    <div class="mx-auto w-full max-w-3xl p-6 ">
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
              class="btn join-item gap-1.5 border-base-300 btn-sm"
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
      </SettingsGroup>

      <!--
        Reduce animation used to sit under Appearance, beside the theme. It is
        not a matter of taste — it is the same control the OS exposes under
        accessibility, and it belongs with the others rather than filed as
        decoration.
      -->
      <SettingsGroup title="Accessibility">
        <SettingsRow label="Interface scale">
          <template #description>
            Resizes everything — text, rows, the rail and the controls — not
            just the type. Ctrl and + or − steps it, Ctrl and 0 returns to 100%.
          </template>

          <div class="join">
            <button
              v-for="step in uiScaleSteps"
              :key="step"
              type="button"
              class="btn join-item border-base-300 font-mono tabular-nums btn-sm"
              :class="themeStore.uiScale === step ? 'btn-primary' : ''"
              :aria-pressed="themeStore.uiScale === step"
              :aria-label="`Interface scale ${step} percent`"
              @click="themeStore.setUiScale(step)"
            >
              {{ step }}%
            </button>
          </div>
        </SettingsRow>

        <SettingsRow
          label="Reduce animation"
          label-for="settings-motion"
        >
          <template #description>
            Turns off panel transitions, the rail's resize and progress easing.
            If your system already asks for reduced motion, that is honoured
            either way.
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
        The first-run screen is the only place the Install/Publish choice is
        made, and once made it never showed again — so the repository and folder
        captured beside it became unreachable as a set, and anyone who picked
        the wrong side on launch had no way back to that screen.
      -->
      <SettingsGroup title="Setup">
        <!--
          CEMM finds this folder from CurseForge's own settings, which works on
          Windows and does not exist to find on Linux — CurseForge ships no
          official build there. This row is how the library works anyway, and
          how anyone keeping packs outside the default location points at them.
        -->
        <SettingsRow label="Modpack instances folder">
          <template #description>
            Where CEMM looks for your modpacks.
            <template v-if="packsStore.instancesDirOverride.length > 0">
              Currently
              <span class="font-mono">{{ packsStore.instancesDirOverride }}</span>.
            </template>
            <template v-else-if="packsStore.library?.instancesDir">
              Found automatically at
              <span class="font-mono">{{ packsStore.library.instancesDir }}</span>.
            </template>
            <template v-else>
              Not found automatically — choose it here.
            </template>
          </template>

          <button
            v-if="packsStore.instancesDirOverride.length > 0"
            type="button"
            class="btn btn-ghost btn-sm"
            @click="resetInstancesDir"
          >
            Use CurseForge's
          </button>
          <button
            type="button"
            class="btn gap-1.5 border-base-300 btn-sm"
            @click="chooseInstancesDir"
          >
            <Icon
              name="mdi:folder-open-outline"
              size="1rem"
              aria-hidden="true"
            />
            Choose
          </button>
        </SettingsRow>

        <SettingsRow label="First-time setup">
          <template #description>
            Reopens the screen CEMM showed on first launch, where you chose
            whether you install updates or publish them. Your repository and
            modpack folder are kept and filled in for you.
          </template>

          <button
            type="button"
            class="btn gap-1.5 border-base-300 btn-sm"
            @click="handleRerunSetup"
          >
            <Icon
              name="mdi:restart"
              size="1rem"
              aria-hidden="true"
            />
            Run again
          </button>
        </SettingsRow>
      </SettingsGroup>

      <!--
        Updates was its own tab holding a single version string and a single
        button, and About was a tab of static text that is not a setting at all.
        They are the same subject, so they are now one block.
      -->
      <SettingsGroup title="About">
        <div class="flex items-start gap-4 p-4 ">
          <BrandMark class="mt-0.5 size-9 shrink-0 text-primary" />

          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-baseline gap-x-2.5 gap-y-1">
              <p class="text-base font-bold">
                CEMM
              </p>
              <p class="font-mono text-xs text-accent tabular-nums">
                v{{ appVersion ?? packageVersion }}
              </p>
            </div>

            <p class="mt-1.5 text-sm/relaxed  text-base-content/65">
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
                class="inline-flex link items-center gap-1 text-primary link-hover"
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
            class="btn gap-1.5 border-base-300 btn-sm"
            :disabled="checking"
            @click="handleCheckForUpdates"
          >
            <span
              v-if="checking"
              class="loading loading-xs loading-spinner"
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
import { UI_SCALE_STEPS } from '~/stores/theme'
import pkg from '~~/package.json'

const updater = useUpdater()
const themeStore = useThemeStore()
const appStore = useAppStore()
const packsStore = usePacksStore()
const { notify } = useNotify()
const { selectDirectory } = useTauri()

const packageVersion = pkg.version
const lastUpdateCheck = ref('')
const checking = ref(false)
const appVersion = ref<string | null>(null)

const uiScaleSteps = UI_SCALE_STEPS

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

/**
 * Deliberately not behind a confirmation. Nothing is destroyed — the mode is
 * re-asked, the repository and folder are carried into the screen already
 * filled in, and the rail is still there to navigate away with.
 */
const chooseInstancesDir = async () =>
{
	const dir = await selectDirectory()
	if (dir === null || dir.trim().length === 0) return
	packsStore.setInstancesDirOverride(dir)
	await packsStore.scan(true)
	notify('CEMM will look for your modpacks there.', 'success')
}

/** Hands discovery back to CurseForge rather than clearing the setting to nothing. */
const resetInstancesDir = async () =>
{
	packsStore.setInstancesDirOverride('')
	await packsStore.scan(true)
}

const handleRerunSetup = async () =>
{
	appStore.resetModeChoice()
	await navigateTo('/')
}

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
