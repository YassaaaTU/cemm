<template>
  <div class="min-h-0 flex-1 overflow-y-auto px-6 py-5">
    <header class="mb-4">
      <p class="text-xs text-base-content/50">
        Preferences
      </p>
      <h1 class="mt-1 text-2xl font-bold tracking-tight">
        Settings
      </h1>
    </header>

    <!--
      Radio tabs. The previous build hand-rolled the WAI-ARIA tabs pattern with
      a keydown handler managing arrow/Home/End focus (F-P2-14). A radio group
      gets that keyboard behaviour from the platform, so the same accessibility
      guarantee now costs no code and cannot drift.
    -->
    <div class="tabs tabs-lift">
      <label class="tab gap-1.5 text-sm font-medium">
        <input
          type="radio"
          name="settings-section"
          checked
        />
        <Icon
          name="mdi:github"
          size="0.9375rem"
          aria-hidden="true"
        />
        Repository
      </label>
      <div class="tab-content border-base-300 bg-base-100 p-4">
        <GitHubSettings />
      </div>

      <label class="tab gap-1.5 text-sm font-medium">
        <input
          type="radio"
          name="settings-section"
        />
        <Icon
          name="mdi:palette-outline"
          size="0.9375rem"
          aria-hidden="true"
        />
        Appearance
      </label>
      <div class="tab-content border-base-300 bg-base-100 p-4">
        <div class="space-y-5">
          <fieldset class="fieldset">
            <legend class="fieldset-legend px-0 text-xs font-semibold text-base-content/70">
              Theme
            </legend>
            <p class="mb-2 text-xs text-base-content/60">
              Matching the system is the default, and follows your desktop the
              moment you change it there.
            </p>
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
            <p
              v-if="themeStore.preference === 'system'"
              class="label mt-1 text-xs"
            >
              Currently showing {{ themeStore.isDark ? 'dark' : 'light' }}.
            </p>
          </fieldset>

          <fieldset class="fieldset border-t border-base-300 pt-4">
            <legend class="fieldset-legend px-0 text-xs font-semibold text-base-content/70">
              Motion
            </legend>
            <label class="label cursor-pointer justify-start gap-3">
              <input
                type="checkbox"
                class="toggle toggle-sm"
                :checked="themeStore.motion === 'reduced'"
                @change="themeStore.toggleMotion()"
              />
              <span class="text-sm">Reduce animation</span>
            </label>
            <p class="label text-xs">
              Turns off panel transitions and staggered list reveals. If your system
              already asks for reduced motion, that is honoured regardless of
              this switch.
            </p>
          </fieldset>
        </div>
      </div>

      <label class="tab gap-1.5 text-sm font-medium">
        <input
          type="radio"
          name="settings-section"
        />
        <Icon
          name="mdi:update"
          size="0.9375rem"
          aria-hidden="true"
        />
        Updates
      </label>
      <div class="tab-content border-base-300 bg-base-100 p-4">
        <div class="space-y-4">
          <div class="flex flex-wrap items-center gap-3">
            <div>
              <p class="text-sm font-semibold">
                CEMM {{ appVersion !== null ? `v${appVersion}` : `v${packageVersion}` }}
              </p>
              <p
                v-if="lastUpdateCheck.length > 0"
                class="text-xs text-base-content/60"
              >
                Last checked {{ lastUpdateCheck }}
              </p>
            </div>
            <span class="flex-1" />
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
        </div>
      </div>

      <label class="tab gap-1.5 text-sm font-medium">
        <input
          type="radio"
          name="settings-section"
        />
        <Icon
          name="mdi:information-outline"
          size="0.9375rem"
          aria-hidden="true"
        />
        About
      </label>
      <div class="tab-content border-base-300 bg-base-100 p-4">
        <div class="flex items-start gap-4">
          <BrandMark class="mt-1 size-10 shrink-0 text-primary" />
          <div class="space-y-2">
            <p class="text-lg font-bold">
              CEMM
            </p>
            <p class="text-sm text-base-content/70">
              ChillEcke Modpack Manager. Distributes modifications to an existing
              CurseForge modpack among a group, without republishing the whole pack.
            </p>
            <dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 pt-1 text-xs">
              <dt class="text-base-content/55">
                Version
              </dt>
              <dd class="font-mono tabular-nums text-accent">
                {{ appVersion ?? packageVersion }}
              </dd>
              <dt class="text-base-content/55">
                Licence
              </dt>
              <dd>MIT</dd>
              <dt class="text-base-content/55">
                Author
              </dt>
              <dd>YassaaaTU</dd>
            </dl>
            <a
              href="https://github.com/YassaaaTU/cemm"
              target="_blank"
              rel="noopener noreferrer"
              class="link link-hover inline-flex items-center gap-1.5 pt-1 text-sm text-primary"
            >
              <Icon
                name="mdi:github"
                size="1rem"
                aria-hidden="true"
              />
              Source on GitHub
            </a>
          </div>
        </div>
      </div>
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
