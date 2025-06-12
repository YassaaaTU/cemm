<template>
  <div>
    <h2 class="font-semibold mb-2">
      GitHub Settings
    </h2>
    <div v-if="loading">
      Loading...
    </div>
    <div v-else>
      <form
        class="space-y-4"
        @submit.prevent="saveSettings"
      >
        <div>
          <label
            class="label"
            for="github-repo"
          >GitHub Repository</label>
          <input
            id="github-repo"
            v-model="githubRepo"
            type="text"
            class="input input-bordered w-full"
            placeholder="user/repo"
            autocomplete="off"
          />
        </div>
        <div>
          <label
            class="label"
            for="github-token"
          >GitHub Token</label>
          <input
            id="github-token"
            v-model="githubToken"
            type="password"
            class="input input-bordered w-full"
            placeholder="Personal Access Token"
            autocomplete="off"
          />
          <div class="text-xs text-gray-500 mt-1">
            Your token is stored securely and never sent anywhere except GitHub.
          </div>
        </div>
        <button
          type="submit"
          class="btn btn-primary w-full"
          :disabled="loading"
        >
          Save Settings
        </button>
        <div v-if="loading">
          Saving...
        </div>        <div
          v-if="error"
          class="mt-4 text-red-600 text-sm"
        >
          {{ error }}
        </div>
        <div
          v-else-if="successMessage"
          class="mt-4 text-green-600 text-sm"
        >
          {{ successMessage }}
        </div>
      </form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'

import { useSecureStorage } from '~/composables/useSecureStorage'
import { useAppStore } from '~/stores/app'

const appStore = useAppStore()
const { setSecure, getSecure } = useSecureStorage()
const logger = usePinoLogger()

// Use computed for githubRepo to ensure reactivity and persistence
const githubRepo = computed({
	get: () => appStore.githubRepo,
	set: (val: string) =>
	{
		appStore.githubRepo = val
	}
})
const githubToken = ref('')
const tokenSaved = ref(false)
const loading = ref(false)
const error = ref('')
const successMessage = ref('')

onMounted(async () =>
{
	loading.value = true
	error.value = ''
	const t0 = performance.now()
	try
	{
		logger.info('Loading GitHub settings...')
		logger.info('Current store githubRepo value', { repo: appStore.githubRepo })

		// Load token from secure storage
		const token = await getSecure('cemm_github_token')
		githubToken.value = token ?? ''
		tokenSaved.value = githubToken.value.length > 0

		logger.info('GitHub settings loaded', {
			hasToken: tokenSaved.value,
			hasRepo: githubRepo.value.length > 0,
			repoValue: githubRepo.value
		})
	}
	catch (err)
	{
		logger.error('Failed to load GitHub settings')
		logger.error(err)
		error.value = 'Failed to load settings'
	}
	finally
	{
		loading.value = false
		const t1 = performance.now()
		logger.info('GitHub settings load time (ms)', { duration: t1 - t0 })
	}
})

const saveSettings = async () =>
{
	loading.value = true
	error.value = ''
	successMessage.value = ''
	const t0 = performance.now()
	try
	{
		logger.info('Saving GitHub settings...')
		logger.info('Before save - githubRepo value', { repo: githubRepo.value })

		// Validate inputs
		if (!githubRepo.value.trim())
		{
			throw new Error('GitHub repository is required')
		}
		if (!githubToken.value.trim())
		{
			throw new Error('GitHub token is required')
		}

		// Save repo to app store (persisted automatically)
		appStore.githubRepo = githubRepo.value.trim()

		logger.info('After save - store githubRepo value', {
			repo: appStore.githubRepo,
			computedRepo: githubRepo.value
		})

		// Save token to secure storage
		await setSecure('cemm_github_token', githubToken.value.trim())

		tokenSaved.value = true
		successMessage.value = 'Settings saved successfully!'

		logger.info('GitHub settings saved successfully')

		// Clear success message after 3 seconds
		setTimeout(() =>
		{
			successMessage.value = ''
		}, 3000)
	}
	catch (err)
	{
		const errorMsg = err instanceof Error ? err.message : 'Failed to save settings'
		logger.error('Failed to save GitHub settings')
		logger.error(err)
		error.value = errorMsg
	}
	finally
	{
		loading.value = false
		const t1 = performance.now()
		logger.info('GitHub settings save time (ms)', { duration: t1 - t0 })
	}
}
// Next step: optimize useSecureStorage if timing logs show slow performance
</script>
