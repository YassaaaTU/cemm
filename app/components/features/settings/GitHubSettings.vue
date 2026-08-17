<template>
  <div>
    <div
      v-if="loading"
      class="flex items-center gap-2 py-4 text-sm text-base-content/60"
    >
      <span
        class="loading loading-spinner loading-sm"
        aria-hidden="true"
      />
      Loading settings…
    </div>

    <form
      v-else
      class="space-y-5"
      @submit.prevent="saveSettings"
    >
      <fieldset class="fieldset">
        <legend class="fieldset-legend px-0 text-[0.6875rem] font-bold uppercase tracking-[0.16em] text-base-content/70">
          Repository
        </legend>

        <label
          class="label"
          for="settings-github-repo"
        >
          Where updates are published
        </label>
        <label
          class="input input-sm w-full max-w-md border-base-300 bg-base-100 font-mono text-xs"
          :class="fieldErrors.repo !== undefined ? 'border-error' : ''"
        >
          <Icon
            name="mdi:github"
            size="0.9375rem"
            class="shrink-0 text-base-content/40"
            aria-hidden="true"
          />
          <input
            id="settings-github-repo"
            v-model="githubRepo"
            type="text"
            class="grow text-accent"
            placeholder="owner/repository"
            spellcheck="false"
            autocomplete="off"
            :aria-invalid="fieldErrors.repo !== undefined"
            :aria-describedby="fieldErrors.repo !== undefined ? 'settings-github-repo-error' : undefined"
          />
        </label>
        <p
          v-if="fieldErrors.repo !== undefined"
          id="settings-github-repo-error"
          class="label text-xs text-error"
        >
          {{ fieldErrors.repo }}
        </p>
        <p
          v-else
          class="label text-xs"
        >
          Everyone in your group uses the same repository, for example
          <span class="font-mono text-accent">YassaaaTU/cemm-updates</span>.
        </p>
      </fieldset>

      <fieldset class="fieldset border-t border-base-300 pt-4">
        <legend class="fieldset-legend px-0 text-[0.6875rem] font-bold uppercase tracking-[0.16em] text-base-content/70">
          Access token
        </legend>

        <label
          class="label"
          for="settings-github-token"
        >
          Personal access token
          <span class="ml-1 font-normal text-base-content/50">— only needed to publish</span>
        </label>
        <label class="input input-sm w-full max-w-md border-base-300 bg-base-100 font-mono text-xs">
          <Icon
            name="mdi:key-variant"
            size="0.9375rem"
            class="shrink-0 text-base-content/40"
            aria-hidden="true"
          />
          <input
            id="settings-github-token"
            v-model="githubToken"
            :type="tokenVisible ? 'text' : 'password'"
            class="grow text-accent"
            placeholder="ghp_…"
            spellcheck="false"
            autocomplete="off"
          />
          <button
            type="button"
            class="btn btn-ghost btn-xs shrink-0 px-1"
            :aria-label="tokenVisible ? 'Hide token' : 'Show token'"
            @click="tokenVisible = !tokenVisible"
          >
            <Icon
              :name="tokenVisible ? 'mdi:eye-off-outline' : 'mdi:eye-outline'"
              size="0.9375rem"
              aria-hidden="true"
            />
          </button>
        </label>

        <p class="label text-xs">
          <template v-if="tokenSaved">
            <Icon
              name="mdi:lock-check-outline"
              size="0.875rem"
              class="text-success"
              aria-hidden="true"
            />
            Stored in your operating system's keyring, not in this app's files.
            Clear the field and save to delete it.
          </template>
          <template v-else>
            If you only install updates, leave this empty — you do not need a token.
          </template>
        </p>
      </fieldset>

      <div class="flex items-center gap-3 border-t border-base-300 pt-4">
        <button
          type="submit"
          class="btn btn-primary btn-sm gap-1.5"
          :disabled="loading"
        >
          <Icon
            name="mdi:content-save-outline"
            size="1rem"
            aria-hidden="true"
          />
          Save
        </button>
      </div>
    </form>
  </div>
</template>

<script setup lang="ts">
import { isValidGithubRepo } from '~/utils/githubRepo'

const appStore = useAppStore()
const { setSecure, getSecure, removeSecure } = useSecureStorage()
const { $logger: logger } = useNuxtApp()
const { notify } = useNotify()

const githubRepo = computed({
	get: () => appStore.githubRepo,
	set: (val: string) =>
	{
		appStore.githubRepo = val
	}
})
const githubToken = ref('')
const tokenVisible = ref(false)
const tokenSaved = ref(false)
const loading = ref(false)
const fieldErrors = ref<{ repo?: string }>({})

onMounted(async () =>
{
	loading.value = true
	const t0 = performance.now()
	try
	{
		logger.info('Loading GitHub settings...')

		const token = await getSecure('cemm_github_token')
		githubToken.value = token ?? ''
		tokenSaved.value = githubToken.value.length > 0

		logger.info({
			hasToken: tokenSaved.value,
			hasRepo: githubRepo.value.length > 0
		}, 'GitHub settings loaded')
	}
	catch (err)
	{
		logger.error('Failed to load GitHub settings')
		logger.error(err)
		notify('Could not load your saved settings.', 'error')
	}
	finally
	{
		loading.value = false
		const t1 = performance.now()
		logger.info({ duration: t1 - t0 }, 'GitHub settings load time (ms)')
	}
})

const saveSettings = async () =>
{
	loading.value = true
	fieldErrors.value = {}
	const t0 = performance.now()
	try
	{
		logger.info('Saving GitHub settings...')

		if (!githubRepo.value.trim())
		{
			fieldErrors.value.repo = 'Enter the repository your group uses.'
			throw new Error('GitHub repository is required')
		}

		if (!isValidGithubRepo(githubRepo.value))
		{
			fieldErrors.value.repo = 'Use the owner/repository form, for example YassaaaTU/cemm-updates.'
			throw new Error('Invalid repository format')
		}

		appStore.githubRepo = githubRepo.value.trim()

		if (githubToken.value.trim())
		{
			await setSecure('cemm_github_token', githubToken.value.trim())
			tokenSaved.value = true
		}
		else
		{
			// Clearing the field should delete the credential, not store an empty
			// string under it (removeSecure existed but nothing called it). The OS
			// keyring rejects deleting an entry that was never set — harmless here,
			// since "no token stored" is the desired end state either way.
			try
			{
				await removeSecure('cemm_github_token')
			}
			catch (removeError)
			{
				logger.debug({ error: removeError }, 'No stored GitHub token to remove')
			}
			tokenSaved.value = false
		}
		notify('Settings saved.', 'success')

		logger.info('GitHub settings saved successfully')
	}
	catch (err)
	{
		const errorMsg = err instanceof Error ? err.message : 'Could not save your settings.'
		logger.error('Failed to save GitHub settings')
		logger.error(err)
		// Field-level problems are already shown next to the field itself, so
		// repeating them in the banner would say the same thing twice.
		// Field-level problems are already shown next to the field itself, so only
		// a non-field failure is worth a toast.
		if (Object.keys(fieldErrors.value).length === 0) notify(errorMsg, 'error')
	}
	finally
	{
		loading.value = false
		const t1 = performance.now()
		logger.info({ duration: t1 - t0 }, 'GitHub settings save time (ms)')
	}
}
</script>
