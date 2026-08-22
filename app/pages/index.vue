<template>
  <div class="flex min-h-0 flex-1 items-center overflow-y-auto px-6 py-8">
    <div class="mx-auto w-full max-w-2xl">
      <!-- Where the settings that never change between updates get captured —
           which is what lets the workspaces be a single screen instead of a
           wizard that re-asks them every time. Shown unprompted on a first run,
           and reachable again from Settings > Setup afterwards.

           It no longer opens by asking which counter you are on. That question
           is answered better on a pack card, where there is a pack in hand to
           answer it about. -->
      <header class="mb-6">
        <p class="text-xs text-base-content/50">
          {{ returning ? 'Setup' : 'First use' }}
        </p>
        <h1 class="mt-1 text-2xl font-bold tracking-tight">
          {{ returning ? 'Update your setup' : 'One-time setup' }}
        </h1>
        <p class="mt-2 max-w-lg text-sm/relaxed  text-base-content/65">
          CEMM moves modpack changes between the person who makes them and
          everyone who plays with them. These settings stay saved, so from now
          on you start from your packs and pick what to do with one.
        </p>
      </header>

      <div class="space-y-5 rounded-box border border-base-300 bg-base-200 p-5">
        <fieldset class="fieldset">
          <label
            class="label text-sm font-medium text-base-content"
            for="setup-repo"
          >
            GitHub repository
          </label>
          <label class="input w-full max-w-md border-base-300 bg-base-100 font-mono text-xs input-sm">
            <Icon
              name="mdi:github"
              size="0.9375rem"
              class="shrink-0 text-base-content/40"
              aria-hidden="true"
            />
            <input
              id="setup-repo"
              v-model="repo"
              type="text"
              class="grow"
              placeholder="owner/repository"
              spellcheck="false"
              autocomplete="off"
            />
          </label>
          <!-- Not `.label`: daisyUI sets `white-space: nowrap` on it, which is
               right for a field's name and wrong for a sentence — the text ran
               straight out of the card instead of wrapping inside it. -->
          <p class="mt-1 text-xs/relaxed text-base-content/60">
            The same one for everyone in your group — where updates get
            published and where they get fetched from. Your admin will tell you
            which to use.
          </p>
        </fieldset>

        <!-- Kept, but no longer required: opening a pack from Your packs sets
             the destination too, and does it with the pack in hand. This is the
             default for anyone who installs into the same folder every time. -->
        <fieldset class="fieldset border-t border-base-300 pt-4">
          <p class="label text-sm font-medium text-base-content">
            Your modpack folder <span class="font-normal text-base-content/50">(optional)</span>
          </p>
          <PathSelector
            type="directory"
            title="Select modpack directory"
            hint="The folder containing your modpack — the one with a mods folder inside it."
            :model-value="folder"
            @update:model-value="onFolder"
            @error="(message: string) => notify(`Could not use that folder: ${message}`, 'error')"
          />
          <p class="mt-1 text-xs/relaxed text-base-content/60">
            Where updates install by default. Installing a pack from Your packs
            sets this as well, so it is safe to leave empty.
          </p>
        </fieldset>

        <div class="flex items-center gap-3 border-t border-base-300 pt-4">
          <div class="flex-1" />
          <button
            type="button"
            class="btn btn-sm"
            @click="finish"
          >
            Skip for now
          </button>
          <button
            type="button"
            class="btn btn-primary btn-sm"
            :disabled="!canFinish"
            @click="finish"
          >
            Get started
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { isValidGithubRepo } from '~/utils/githubRepo'

const appStore = useAppStore()
const { notify } = useNotify()

const repo = ref(appStore.githubRepo)
const folder = ref(appStore.modpackPath)

/**
 * This screen is reachable again from Settings, so it is no longer only ever a
 * first run. Anything already configured means the user has been here before,
 * and calling that "First use" would be a lie on the one screen whose job is to
 * be plain about what it is asking.
 */
const returning = computed(
	() => appStore.githubRepo.length > 0 || appStore.modpackPath.length > 0
)

/**
 * Only the repository gates the primary action. The folder has a second, better
 * source now — a pack card — so requiring it here would block setup on a
 * decision the library makes for you.
 */
const canFinish = computed(() => isValidGithubRepo(repo.value))

const onFolder = (value: string | string[] | null) =>
{
	const single = Array.isArray(value) ? value[0] : value
	folder.value = single ?? ''
}

/**
 * Skipping is allowed on purpose: a player who has not been given the repository
 * yet should still reach their packs, where both settings are editable inline.
 */
const finish = async () =>
{
	if (isValidGithubRepo(repo.value))
	{
		appStore.githubRepo = repo.value.trim()
	}
	if (folder.value.trim().length > 0)
	{
		appStore.modpackPath = folder.value.trim()
	}

	appStore.completeSetup()
	await navigateTo('/packs')
}

// A returning user never reaches this screen at all: setup.global.ts redirects
// them to their packs before the route resolves.
</script>
