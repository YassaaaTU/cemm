<template>
  <div class="min-h-screen flex flex-col items-stretch justify-stretch bg-base-100">
    <div class="flex-1 flex flex-col sm:flex-row">
      <!-- Admin Mode Button -->
      <button
        class="landing-button bg-primary hover:bg-accent group"
        @click="goToApp('admin')"
      >
        <Icon
          name="mdi:shield-crown"
          size="3rem"
          class="mb-4 group-hover:scale-125 transition-all duration-300 lTextAdmin"
        />
        <span class="mb-2 lTextAdmin">
          Admin Mode
        </span>
        <span class="lTextAdmin text-sm">
          For managing modpacks, users, and settings.
        </span>
        <span class="lTextAdmin text-sm">
          Full control with advanced features. Not installing modpack updates.
        </span>
      </button>
      <!-- User Mode Button -->
      <button
        class="landing-button bg-secondary hover:bg-accent group"
        @click="goToApp('user')"
      >
        <Icon
          name="mdi:account"
          size="3rem"
          class="mb-4 group-hover:scale-125 transition-all duration-300 lTextUser"
        />
        <span class="mb-2 lTextUser">
          User Mode
        </span>
        <span class="lTextUser text-sm">
          For installing modpack updates using a UUID code from your admin.
        </span>
        <span class="lTextUser text-sm">
          Usually used by players to get the latest updates from their admin.
        </span>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useAppStore } from '~/stores/app'

const appStore = useAppStore()
const router = useRouter()

function goToApp(mode: 'admin' | 'user')
{
	appStore.mode = mode
	router.push({ name: 'dashboard' })
}

// use the landing layout
definePageMeta({
	layout: 'landing'
})
</script>
