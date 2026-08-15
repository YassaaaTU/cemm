<template>
  <div class="github-settings">
    <div
      v-if="loading"
      class="github-settings__loading"
    >
      <span class="loading loading-spinner loading-sm" />
      <span>Loading settings...</span>
    </div>

    <form
      v-else
      class="github-settings__form"
      @submit.prevent="saveSettings"
    >
      <div class="github-settings__field">
        <label
          class="label"
          for="github-repo"
        >
          <span class="label-text">Modpack Updates Repository</span>
        </label>
        <input
          id="github-repo"
          v-model="githubRepo"
          type="text"
          class="input input-bordered w-full"
          :class="{ 'input-error': !!fieldErrors.repo }"
          placeholder="user/repo (e.g., john/my-modpack-updates)"
          autocomplete="off"
        />
        <div class="label">
          <span
            v-if="fieldErrors.repo"
            class="label-text-alt text-error"
          >{{ fieldErrors.repo }}</span>
          <span
            v-else
            class="label-text-alt"
          >Required: The GitHub repository where modpack updates will be stored (not the app repository)</span>
        </div>
      </div>

      <div class="github-settings__field">
        <label
          class="label"
          for="github-token"
        >
          <span class="label-text">GitHub Token (Optional)</span>
        </label>
        <input
          id="github-token"
          v-model="githubToken"
          type="password"
          class="input input-bordered w-full"
          placeholder="Personal Access Token (optional for public repos)"
          autocomplete="off"
        />
        <div class="label">
          <span class="label-text-alt">Optional: Required only for private repositories or enhanced rate limits</span>
        </div>
      </div>

      <div class="github-settings__actions">
        <button
          type="submit"
          class="btn btn-primary w-full"
          :disabled="loading || !githubRepo.trim()"
        >
          <span
            v-if="loading"
            class="loading loading-spinner loading-xs"
          />
          <Icon
            v-else
            name="mdi:content-save"
            size="1.1rem"
          />
          <span>{{ loading ? 'Saving...' : 'Save Settings' }}</span>
        </button>
      </div>

      <div
        v-if="error"
        class="alert alert-error github-settings__alert"
        role="alert"
      >
        <Icon name="mdi:alert-circle" />
        <span>{{ error }}</span>
        <button
          class="btn btn-sm btn-ghost"
          aria-label="Dismiss error message"
          @click="error = ''"
        >
          <Icon
            name="mdi:close"
            size="1.2rem"
          />
        </button>
      </div>

      <div
        v-else-if="successMessage"
        class="alert alert-success github-settings__alert"
        role="alert"
      >
        <Icon name="mdi:check-circle" />
        <span>{{ successMessage }}</span>
        <button
          class="btn btn-sm btn-ghost"
          aria-label="Dismiss success message"
          @click="successMessage = ''"
        >
          <Icon
            name="mdi:close"
            size="1.2rem"
          />
        </button>
      </div>
    </form>
  </div>
</template>

<script setup lang="ts">
const appStore = useAppStore()
const { setSecure, getSecure } = useSecureStorage()
const { $logger: logger } = useNuxtApp()

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
const fieldErrors = ref<{ repo?: string }>({})

onMounted(async () =>
{
	loading.value = true
	error.value = ''
	const t0 = performance.now()
	try
	{
		logger.info('Loading GitHub settings...')
		logger.info('Current store githubRepo value', { repo: appStore.githubRepo })

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
	fieldErrors.value = {}
	const t0 = performance.now()
	try
	{
		logger.info('Saving GitHub settings...')
		logger.info('Before save - githubRepo value', { repo: githubRepo.value })

		if (!githubRepo.value.trim())
		{
			fieldErrors.value.repo = 'GitHub repository is required'
			throw new Error('GitHub repository is required')
		}

		const repoPattern = /^[a-zA-Z0-9._-]+\/[a-zA-Z0-9._-]+$/
		if (!repoPattern.test(githubRepo.value.trim()))
		{
			fieldErrors.value.repo = 'Invalid format. Use "owner/repo" (e.g., YassaaaTU/cemm-updates)'
			throw new Error('Invalid repository format')
		}

		appStore.githubRepo = githubRepo.value.trim()

		logger.info('After save - store githubRepo value', {
			repo: appStore.githubRepo,
			computedRepo: githubRepo.value
		})

		if (githubToken.value.trim())
		{
			await setSecure('cemm_github_token', githubToken.value.trim())
			tokenSaved.value = true
		}
		else
		{
			await setSecure('cemm_github_token', '')
			tokenSaved.value = false
		}
		successMessage.value = 'Settings saved successfully!'

		logger.info('GitHub settings saved successfully')

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
</script>

<style scoped>
.github-settings__loading {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  color: var(--color-text-secondary);
  font-size: var(--text-body-md-size);
  padding: var(--space-4) 0;
}

.github-settings__form {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.github-settings__field {
  display: flex;
  flex-direction: column;
}

.github-settings__actions {
  display: flex;
  gap: var(--space-3);
}

.github-settings__alert {
  margin-top: var(--space-2);
}
</style>
