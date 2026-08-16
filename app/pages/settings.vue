<template>
  <div class="settings-page">
    <h1 class="settings-page__title">
      Settings
    </h1>

    <div
      class="settings-page__tabs"
      role="tablist"
      aria-label="Settings sections"
    >
      <button
        v-for="tab in tabs"
        :id="`tab-${tab.key}`"
        :key="tab.key"
        ref="tabRefs"
        class="settings-page__tab"
        :class="{ 'settings-page__tab--active': activeTab === tab.key }"
        role="tab"
        :aria-selected="activeTab === tab.key"
        :aria-controls="`tabpanel-${tab.key}`"
        :tabindex="activeTab === tab.key ? 0 : -1"
        @click="activeTab = tab.key"
        @keydown="handleTabKeydown"
      >
        <Icon
          :name="tab.icon"
          size="1.1rem"
        />
        <span>{{ tab.label }}</span>
      </button>
    </div>

    <div class="settings-page__content">
      <section
        v-if="activeTab === 'github'"
        id="tabpanel-github"
        class="settings-page__section animate-fade-in-up"
        role="tabpanel"
        aria-labelledby="tab-github"
        tabindex="0"
      >
        <div class="card bg-base-200 shadow-md border border-base-300">
          <div class="card-body">
            <div class="settings-page__section-content">
              <h2 class="settings-page__section-title">
                Modpack Updates Repository
              </h2>
              <p class="settings-page__section-desc">
                Configure the GitHub repository where your modpack updates are stored (e.g., YassaaaTU/cemm-updates).
              </p>
              <GitHubSettings />
            </div>
          </div>
        </div>
      </section>

      <section
        v-if="activeTab === 'appearance'"
        id="tabpanel-appearance"
        class="settings-page__section animate-fade-in-up"
        role="tabpanel"
        aria-labelledby="tab-appearance"
        tabindex="0"
      >
        <div class="card bg-base-200 shadow-md border border-base-300">
          <div class="card-body">
            <div class="settings-page__section-content">
              <h2 class="settings-page__section-title">
                Appearance
              </h2>
              <p class="settings-page__section-desc">
                Customize the look and feel of the application.
              </p>

              <div class="settings-page__field">
                <label class="settings-page__field-label">Theme</label>
                <div class="join">
                  <button
                    v-for="opt in themeOptions"
                    :key="opt.value"
                    class="btn join-item"
                    :class="{ 'btn-primary': currentTheme === opt.value }"
                    @click="setTheme(opt.value as 'nord' | 'dracula')"
                  >
                    <Icon
                      v-if="opt.icon"
                      :name="opt.icon"
                      size="1.1rem"
                    />
                    {{ opt.label }}
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      <section
        v-if="activeTab === 'updates'"
        id="tabpanel-updates"
        class="settings-page__section animate-fade-in-up"
        role="tabpanel"
        aria-labelledby="tab-updates"
        tabindex="0"
      >
        <div class="card bg-base-200 shadow-md border border-base-300">
          <div class="card-body">
            <div class="settings-page__section-content">
              <h2 class="settings-page__section-title">
                Application Updates
              </h2>
              <p class="settings-page__section-desc">
                Check for the latest version of CEMM from the app repository (YassaaaTU/cemm).
              </p>

              <div class="settings-page__update-row">
                <button
                  class="btn btn-primary"
                  :disabled="updater.isChecking.value"
                  @click="handleCheckForUpdates"
                >
                  <span
                    v-if="updater.isChecking.value"
                    class="loading loading-spinner loading-xs"
                  />
                  <Icon
                    v-else
                    name="mdi:update"
                    size="1.1rem"
                  />
                  <span>{{ updater.isChecking.value ? 'Checking...' : 'Check for Updates' }}</span>
                </button>
                <span
                  v-if="lastUpdateCheck"
                  class="settings-page__update-timestamp"
                >
                  Last checked: {{ lastUpdateCheck }}
                </span>
              </div>

              <div
                v-if="updateStatus"
                class="alert settings-page__update-status"
                :class="`alert-${updateStatus.type}`"
                role="alert"
              >
                <Icon :name="updateStatus.type === 'success' ? 'mdi:check-circle' : updateStatus.type === 'error' ? 'mdi:alert-circle' : 'mdi:information'" />
                <span>{{ updateStatus.message }}</span>
              </div>
            </div>
          </div>
        </div>
      </section>

      <section
        v-if="activeTab === 'about'"
        id="tabpanel-about"
        class="settings-page__section animate-fade-in-up"
        role="tabpanel"
        aria-labelledby="tab-about"
        tabindex="0"
      >
        <div class="card bg-base-200 shadow-md border border-base-300">
          <div class="card-body">
            <div class="settings-page__section-content">
              <h2 class="settings-page__section-title">
                About CEMM
              </h2>
              <p class="settings-page__section-desc">
                Custom Edition Modpack Manager. A tool for managing and distributing Minecraft modpack updates.
              </p>

              <div class="settings-page__about-info">
                <div class="settings-page__about-row">
                  <Icon
                    name="mdi:tag"
                    size="1.1rem"
                  />
                  <span class="settings-page__about-label">Version</span>
                  <span class="settings-page__about-value">{{ appVersion ?? 'Unavailable' }}</span>
                </div>
                <div class="settings-page__about-row">
                  <Icon
                    name="mdi:github"
                    size="1.1rem"
                  />
                  <span class="settings-page__about-label">Repository</span>
                  <a
                    href="https://github.com/YassaaaTU/cemm"
                    target="_blank"
                    rel="noopener noreferrer"
                    class="settings-page__about-link"
                  >
                    GitHub Repository
                  </a>
                </div>
                <div class="settings-page__about-row">
                  <Icon
                    name="mdi:file-document"
                    size="1.1rem"
                  />
                  <span class="settings-page__about-label">License</span>
                  <a
                    href="https://github.com/YassaaaTU/cemm/blob/main/LICENSE"
                    target="_blank"
                    rel="noopener noreferrer"
                    class="settings-page__about-link"
                  >
                    MIT License
                  </a>
                </div>
              </div>

              <div class="settings-page__about-tech">
                <p class="settings-page__about-tech-label">
                  Built with
                </p>
                <div class="settings-page__about-tech-list">
                  <span class="settings-page__about-tech-item">Tauri</span>
                  <span class="settings-page__about-tech-item">Vue</span>
                  <span class="settings-page__about-tech-item">Nuxt</span>
                  <span class="settings-page__about-tech-item">Tailwind CSS</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useThemeStore } from '~/stores/theme'

const updater = useUpdater()
const themeStore = useThemeStore()

const activeTab = ref<'github' | 'appearance' | 'updates' | 'about'>('github')
const lastUpdateCheck = ref<string>('')
const updateStatus = ref<{ type: 'success' | 'error' | 'info', message: string } | null>(null)
const appVersion = ref<string | null>(null)

const tabs = [
	{ key: 'github', label: 'GitHub', icon: 'mdi:github' },
	{ key: 'appearance', label: 'Appearance', icon: 'mdi:palette' },
	{ key: 'updates', label: 'Updates', icon: 'mdi:update' },
	{ key: 'about', label: 'About', icon: 'mdi:information' }
] as const

// WAI-ARIA tabs pattern: only the active tab sits in the normal tab order
// (:tabindex on each button above), and arrow keys move both focus and
// selection between tabs — previously there was no tabpanel/aria-controls
// pairing and no keyboard way to move between tabs at all (F-P2-14).
const tabRefs = ref<HTMLButtonElement[]>([])

function handleTabKeydown(e: KeyboardEvent)
{
	const currentIndex = tabs.findIndex((t) => t.key === activeTab.value)
	let nextIndex: number

	switch (e.key)
	{
		case 'ArrowRight':
		case 'ArrowDown':
			nextIndex = (currentIndex + 1) % tabs.length
			break
		case 'ArrowLeft':
		case 'ArrowUp':
			nextIndex = (currentIndex - 1 + tabs.length) % tabs.length
			break
		case 'Home':
			nextIndex = 0
			break
		case 'End':
			nextIndex = tabs.length - 1
			break
		default:
			return
	}

	e.preventDefault()
	const nextTab = tabs[nextIndex]
	if (nextTab === undefined) return

	activeTab.value = nextTab.key
	nextTick(() =>
	{
		tabRefs.value[nextIndex]?.focus()
	})
}

const currentTheme = computed(() => themeStore.current)
const setTheme = (theme: 'nord' | 'dracula') =>
{
	themeStore.setTheme(theme)
}
const themeOptions = [
	{ value: 'nord', label: 'Light', icon: 'mdi:white-balance-sunny' },
	{ value: 'dracula', label: 'Dark', icon: 'mdi:moon-waning-crescent' }
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
		{ /* fallback */ }
	}
})

const handleCheckForUpdates = async () =>
{
	updateStatus.value = null
	try
	{
		const result = await updater.checkForUpdates()
		lastUpdateCheck.value = new Date().toLocaleString()
		updateStatus.value = result !== null
			? { type: 'info', message: `Update available: v${result.version}. The update dialog will appear automatically.` }
			: { type: 'success', message: `You're running the latest version (v${appVersion.value ?? ''}).` }
	}
	catch (error)
	{
		updateStatus.value = { type: 'error', message: `Failed to check for updates: ${error}` }
	}
}

definePageMeta({ layout: 'default' })
</script>

<style scoped>
.settings-page {
  max-width: var(--container-md);
  width: 100%;
  margin: 0 auto;
}

.settings-page__title {
  font-family: var(--font-heading);
  font-size: var(--text-heading-xl-size);
  font-weight: 700;
  color: var(--color-text-primary);
  margin: 0 0 var(--space-6);
  letter-spacing: var(--text-heading-xl-letter-spacing);
}

.settings-page__tabs {
  display: flex;
  gap: var(--space-1);
  border-bottom: 2px solid var(--color-border-subtle);
  margin-bottom: var(--space-6);
  overflow-x: auto;
}

.settings-page__tab {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  border: none;
  background: transparent;
  color: var(--color-text-secondary);
  font-family: var(--font-sans);
  font-size: var(--text-body-md-size);
  font-weight: 500;
  cursor: pointer;
  border-radius: var(--radius-md) var(--radius-md) 0 0;
  transition: all var(--duration-fast) var(--ease-standard);
  white-space: nowrap;
  position: relative;
}

.settings-page__tab:hover {
  color: var(--color-text-primary);
  background: var(--color-surface-hover);
}

.settings-page__tab--active {
  color: var(--color-accent-primary);
  font-weight: 600;
}

.settings-page__tab--active::after {
  content: '';
  position: absolute;
  bottom: -2px;
  left: 0;
  right: 0;
  height: 2px;
  background: var(--color-accent-primary);
}

.settings-page__tab:focus-visible {
  box-shadow: var(--focus-ring-offset);
  outline: none;
}

.settings-page__content {
  min-height: 400px;
}

.settings-page__section {
  animation: fadeInUp var(--duration-smooth) var(--ease-out);
}

.settings-page__section-content {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.settings-page__section-title {
  font-family: var(--font-heading);
  font-size: var(--text-heading-sm-size);
  font-weight: 700;
  color: var(--color-text-primary);
  margin: 0;
}

.settings-page__section-desc {
  font-size: var(--text-body-sm-size);
  color: var(--color-text-secondary);
  margin: 0;
}

.settings-page__field {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.settings-page__field-label {
  font-size: var(--text-body-sm-size);
  font-weight: 600;
  color: var(--color-text-primary);
}

.settings-page__update-row {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  flex-wrap: wrap;
}

.settings-page__update-timestamp {
  font-size: var(--text-body-sm-size);
  color: var(--color-text-tertiary);
}

.settings-page__update-status {
  margin-top: var(--space-2);
}

.settings-page__about-info {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.settings-page__about-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  color: var(--color-text-secondary);
}

.settings-page__about-label {
  font-size: var(--text-body-sm-size);
  font-weight: 600;
  color: var(--color-text-primary);
  min-width: 80px;
}

.settings-page__about-value {
  font-size: var(--text-body-sm-size);
  font-variant-numeric: tabular-nums;
  color: var(--color-text-secondary);
}

.settings-page__about-link {
  font-size: var(--text-body-sm-size);
  color: var(--color-accent-primary);
  text-decoration: none;
  transition: color var(--duration-fast);
}
.settings-page__about-link:hover {
  text-decoration: underline;
}

.settings-page__about-tech {
  margin-top: var(--space-4);
  padding-top: var(--space-4);
  border-top: 1px solid var(--color-border-divider);
}

.settings-page__about-tech-label {
  font-size: var(--text-body-xs-size);
  color: var(--color-text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin: 0 0 var(--space-2);
}

.settings-page__about-tech-list {
  display: flex;
  gap: var(--space-2);
  flex-wrap: wrap;
}

.settings-page__about-tech-item {
  padding: var(--space-1) var(--space-3);
  background: var(--color-surface-overlay);
  border-radius: var(--radius-full);
  font-size: var(--text-body-xs-size);
  color: var(--color-text-secondary);
  font-weight: 500;
}

@keyframes fadeInUp {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
