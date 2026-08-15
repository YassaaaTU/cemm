<template>
  <aside
    class="sidebar"
    role="navigation"
    aria-label="Main sidebar navigation"
  >
    <!-- Brand / App Name -->
    <div class="sidebar__brand">
      <span class="sidebar__brand-mark">C</span>
      <span class="sidebar__brand-text">CEMM</span>
    </div>

    <!-- Mode selector -->
    <div class="sidebar__mode">
      <ModeSelector />
    </div>

    <div class="sidebar__divider" />

    <!-- Navigation links -->
    <nav class="sidebar__nav">
      <button
        v-for="item in navItems"
        :key="item.path"
        class="sidebar__nav-item"
        :class="{ 'sidebar__nav-item--active': route.path === item.path }"
        :aria-label="item.label"
        :aria-current="route.path === item.path ? 'page' : undefined"
        @click="navigateTo(item.path)"
      >
        <Icon
          :name="item.icon"
          size="1.2rem"
          class="sidebar__nav-icon"
        />
        <span class="sidebar__nav-label">{{ item.label }}</span>
      </button>
    </nav>

    <div class="sidebar__spacer" />

    <div class="sidebar__divider" />

    <!-- Theme toggle -->
    <div class="sidebar__theme">
      <div class="join">
        <button
          v-for="opt in themeOptions"
          :key="opt.value"
          class="btn join-item btn-sm"
          :class="{ 'btn-primary': currentTheme === opt.value }"
          :aria-pressed="currentTheme === opt.value"
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
  </aside>
</template>

<script lang="ts" setup>
import { useThemeStore } from '~/stores/theme'

const themeStore = useThemeStore()
const route = useRoute()

const currentTheme = computed(() => themeStore.current)
const setTheme = (theme: 'nord' | 'dracula') =>
{
	themeStore.setTheme(theme)
}

watchEffect(() =>
{
	if (import.meta.client)
	{
		document.documentElement.setAttribute('data-theme', themeStore.current)
	}
})

const navItems = [
	{ label: 'Home', icon: 'mdi:home', path: '/' },
	{ label: 'Dashboard', icon: 'mdi:view-dashboard', path: '/dashboard' },
	{ label: 'Settings', icon: 'mdi:cog', path: '/settings' }
]

const themeOptions = [
	{ value: 'nord', label: 'Light', icon: 'mdi:white-balance-sunny' },
	{ value: 'dracula', label: 'Dark', icon: 'mdi:moon-waning-crescent' }
]
</script>

<style scoped>
.sidebar {
  position: fixed;
  top: 0;
  left: 0;
  height: 100dvh;
  width: 200px;
  background: var(--color-surface-raised);
  border-right: 1px solid var(--color-border-subtle);
  display: flex;
  flex-direction: column;
  padding: var(--space-4) 0;
  z-index: 30;
  overflow-y: auto;
  overflow-x: hidden;
}

.sidebar__brand {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: 0 var(--space-4) var(--space-3);
}

.sidebar__brand-mark {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: var(--radius-md);
  background: var(--color-accent-primary);
  color: var(--color-text-inverse);
  font-family: var(--font-heading);
  font-weight: 700;
  font-size: 1.1rem;
}

.sidebar__brand-text {
  font-family: var(--font-heading);
  font-weight: 700;
  font-size: var(--text-heading-sm-size);
  letter-spacing: -0.02em;
  color: var(--color-text-primary);
}

.sidebar__mode {
  padding: var(--space-2) var(--space-3);
}

.sidebar__divider {
  height: 1px;
  background: var(--color-border-divider);
  margin: var(--space-2) var(--space-4);
}

.sidebar__nav {
  display: flex;
  flex-direction: column;
  padding: var(--space-1) var(--space-3);
  gap: var(--space-1);
}

.sidebar__nav-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  border: none;
  background: transparent;
  color: var(--color-text-secondary);
  font-family: var(--font-sans);
  font-size: var(--text-body-md-size);
  font-weight: 500;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--duration-fast) var(--ease-standard);
  width: 100%;
  text-align: left;
}

.sidebar__nav-item:hover {
  background: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.sidebar__nav-item--active {
  background: var(--color-accent-primary-muted);
  color: var(--color-accent-primary);
  font-weight: 600;
}

.sidebar__nav-item:focus-visible {
  box-shadow: var(--focus-ring-offset);
  outline: none;
}

.sidebar__nav-icon {
  flex-shrink: 0;
}

.sidebar__nav-label {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.sidebar__spacer {
  flex: 1;
}

.sidebar__theme {
  padding: var(--space-2) var(--space-3);
}
</style>
