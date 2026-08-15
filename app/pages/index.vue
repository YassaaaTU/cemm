<template>
  <div class="landing">
    <!-- Gradient mesh background -->
    <div class="landing__bg" />

    <!-- Content -->
    <div class="landing__content">
      <!-- Brand -->
      <div class="landing__brand animate-fade-in-up">
        <div class="landing__brand-mark">
          C
        </div>
        <h1 class="landing__brand-name">
          CEMM
        </h1>
        <p class="landing__brand-tagline">
          Custom Edition Modpack Manager
        </p>
      </div>

      <!-- Mode Selection Cards -->
      <div
        class="landing__cards animate-fade-in-up"
        style="animation-delay: 100ms"
      >
        <!-- Admin Card -->
        <button
          class="landing-card"
          :class="{ 'landing-card--active': hoveredMode === 'admin' }"
          @click="goToApp('admin')"
          @mouseenter="hoveredMode = 'admin'"
          @mouseleave="hoveredMode = ''"
        >
          <Icon
            name="mdi:shield-crown"
            size="2.5rem"
            class="landing-card__icon card-icon card-icon--admin"
          />
          <span class="landing-card__title card-title">Admin</span>
          <span class="landing-card__subtitle">Create & Publish Updates</span>
          <span class="landing-card__desc card-desc">
            Manage modpacks, curate addon lists, and publish updates for your players.
          </span>
        </button>

        <!-- User Card -->
        <button
          class="landing-card"
          :class="{ 'landing-card--active': hoveredMode === 'user' }"
          @click="goToApp('user')"
          @mouseenter="hoveredMode = 'user'"
          @mouseleave="hoveredMode = ''"
        >
          <Icon
            name="mdi:account"
            size="2.5rem"
            class="landing-card__icon card-icon card-icon--user"
          />
          <span class="landing-card__title card-title">User</span>
          <span class="landing-card__subtitle">Receive & Install Updates</span>
          <span class="landing-card__desc card-desc">
            Install modpack updates from your admin using an update code.
          </span>
        </button>
      </div>

      <!-- Version -->
      <div
        v-if="appVersion !== null"
        class="landing__version animate-fade-in-up"
        style="animation-delay: 200ms"
      >
        v{{ appVersion }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useAppStore } from '~/stores/app'

const appStore = useAppStore()
const router = useRouter()

const hoveredMode = ref<'admin' | 'user' | ''>('')
const appVersion = ref<string | null>(null)

function goToApp(mode: 'admin' | 'user')
{
	appStore.mode = mode
	router.push({ name: 'dashboard' })
}

// Auto-navigate if user has already chosen a mode (returning user)
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
			// Not in Tauri - fallback
		}

		// If mode is already set (returning user), go straight to dashboard
		// Commented out for now to always show landing - uncomment to skip:
		// if (appStore.mode && appStore.mode.length > 0) {
		//   router.replace({ name: 'dashboard' })
		// }
	}
})

definePageMeta({
	layout: 'landing'
})
</script>

<style scoped>
.landing {
  position: relative;
  min-height: 100dvh;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
}

/* Gradient mesh background */
.landing__bg {
  position: absolute;
  inset: 0;
  background:
    radial-gradient(ellipse 60% 50% at 10% 20%, rgba(232, 75, 187, 0.12) 0%, transparent 70%),
    radial-gradient(ellipse 50% 60% at 90% 80%, rgba(124, 92, 252, 0.08) 0%, transparent 70%),
    radial-gradient(ellipse 40% 40% at 50% 50%, rgba(0, 212, 170, 0.04) 0%, transparent 60%);
  pointer-events: none;
  z-index: 0;
}

.landing__content {
  position: relative;
  z-index: 1;
  width: 100%;
  max-width: 580px;
  padding: var(--space-8);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-8);
}

/* Brand */
.landing__brand {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-2);
}

.landing__brand-mark {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 56px;
  height: 56px;
  border-radius: var(--radius-lg);
  background: var(--color-accent-primary);
  color: var(--color-text-inverse);
  font-family: var(--font-heading);
  font-weight: 700;
  font-size: 1.5rem;
  margin-bottom: var(--space-2);
  box-shadow: 0 0 0 4px var(--color-accent-primary-muted),
              0 8px 32px rgba(232, 75, 187, 0.25);
}

.landing__brand-name {
  font-family: var(--font-heading);
  font-size: var(--text-heading-xl-size);
  font-weight: 700;
  letter-spacing: -0.03em;
  color: var(--color-text-primary);
  line-height: 1;
}

.landing__brand-tagline {
  font-size: var(--text-body-md-size);
  color: var(--color-text-secondary);
  margin-top: var(--space-1);
}

/* Cards container */
.landing__cards {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-4);
  width: 100%;
}

/* Landing card - extends the base .landing-card from main.css */
.landing-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: var(--space-2);
  padding: var(--space-8) var(--space-6);
  min-height: 220px;
}

.landing-card__subtitle {
  font-size: var(--text-body-md-size);
  font-weight: 600;
  color: var(--color-accent-primary);
  margin-top: var(--space-1);
}

.landing-card--active {
  border-color: var(--color-accent-primary);
  box-shadow: 0 0 0 1px var(--color-accent-primary), var(--elev-2);
}

/* Version */
.landing__version {
  font-size: var(--text-body-xs-size);
  color: var(--color-text-tertiary);
  font-variant-numeric: tabular-nums;
}

/* Responsive */
@media (max-width: 480px) {
  .landing__cards {
    grid-template-columns: 1fr;
  }
  .landing-card {
    min-height: 120px;
    padding: var(--space-6) var(--space-4);
  }
}
</style>
