<template>
  <div class="flex min-h-0 flex-1 items-center overflow-y-auto px-6 py-8">
    <div class="mx-auto w-full max-w-2xl">
      <!-- First run only. This is where the settings that never change between
           updates get captured — which is what lets the workspaces be a single
           screen instead of a wizard that re-asks them every time. -->
      <header class="mb-6">
        <p class="text-xs text-base-content/50">
          First use
        </p>
        <h1 class="mt-1 text-2xl font-bold tracking-tight">
          {{ chosen === null ? 'Which side are you on?' : 'One-time setup' }}
        </h1>
        <p class="mt-2 max-w-lg text-sm leading-relaxed text-base-content/65">
          <template v-if="chosen === null">
            CEMM moves modpack changes between the person who makes them and
            everyone who plays with them. You can switch any time from the rail.
          </template>
          <template v-else>
            These stay saved, so from now on {{ chosen === 'user' ? 'installing an update is just pasting a code' : 'publishing is load, curate, publish' }}.
          </template>
        </p>
      </header>

      <!-- Phase 1 — pick a side -->
      <div
        v-if="chosen === null"
        class="grid gap-3 sm:grid-cols-2"
      >
        <button
          v-for="counter in counters"
          :key="counter.mode"
          type="button"
          class="group rounded-box border border-base-300 bg-base-200 p-4 text-left transition-colors duration-200 ease-[var(--ease-standard)] hover:border-primary/60 hover:bg-base-300 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
          @click="chosen = counter.mode"
        >
          <span class="grid size-10 place-items-center rounded-box bg-primary/15 text-primary">
            <Icon
              :name="counter.icon"
              size="1.25rem"
              aria-hidden="true"
            />
          </span>

          <span class="mt-3 block text-base font-semibold">{{ counter.title }}</span>
          <span class="mt-1 block text-sm leading-relaxed text-base-content/65">
            {{ counter.description }}
          </span>

          <span class="mt-3 flex items-center gap-1.5 text-sm font-medium text-primary">
            {{ counter.action }}
            <Icon
              name="mdi:arrow-right"
              size="0.9375rem"
              class="transition-transform duration-200 ease-[var(--ease-out-quick)] group-hover:translate-x-1"
              aria-hidden="true"
            />
          </span>
        </button>
      </div>

      <!-- Phase 2 — the settings that never change -->
      <div
        v-else
        class="space-y-5 rounded-box border border-base-300 bg-base-200 p-5"
      >
        <fieldset class="fieldset">
          <label
            class="label text-sm font-medium text-base-content"
            for="setup-repo"
          >
            GitHub repository
          </label>
          <label class="input input-sm w-full max-w-md border-base-300 bg-base-100 font-mono text-xs">
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
          <p class="label text-xs">
            <template v-if="chosen === 'user'">
              Your admin will tell you which one to use — it is the same for
              everyone in your group.
            </template>
            <template v-else>
              Where your updates get published. Players point CEMM at the same one.
            </template>
          </p>
        </fieldset>

        <!-- Only the player has a fixed destination; an admin picks the instance
             per publish through the native dialog. -->
        <fieldset
          v-if="chosen === 'user'"
          class="fieldset border-t border-base-300 pt-4"
        >
          <p class="label text-sm font-medium text-base-content">
            Your modpack folder
          </p>
          <PathSelector
            type="directory"
            title="Select modpack directory"
            hint="The folder containing your modpack — the one with a mods folder inside it."
            :model-value="folder"
            @update:model-value="onFolder"
            @error="(message: string) => notify(`Could not use that folder: ${message}`, 'error')"
          />
        </fieldset>

        <div class="flex items-center gap-3 border-t border-base-300 pt-4">
          <button
            type="button"
            class="btn btn-ghost btn-sm"
            @click="chosen = null"
          >
            Back
          </button>
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
import type { AppMode } from '~/stores/app'
import { isValidGithubRepo } from '~/utils/githubRepo'

const appStore = useAppStore()
const { notify } = useNotify()

const chosen = ref<AppMode | null>(null)
const repo = ref(appStore.githubRepo)
const folder = ref(appStore.modpackPath)

const counters: Array<{ mode: AppMode, title: string, description: string, action: string, icon: string }> = [
	{
		mode: 'user',
		title: 'Install an update',
		description: 'Someone sent you a code. Paste it in, see exactly what changes, and apply it to your modpack.',
		action: 'Set this up',
		icon: 'mdi:tray-arrow-down'
	},
	{
		mode: 'admin',
		title: 'Publish an update',
		description: 'You changed the modpack. Package the difference, upload it, and get a code to hand out.',
		action: 'Set this up',
		icon: 'mdi:tray-arrow-up'
	}
]

const canFinish = computed(() =>
{
	if (!isValidGithubRepo(repo.value)) return false
	if (chosen.value === 'user' && folder.value.trim().length === 0) return false
	return true
})

const onFolder = (value: string | string[] | null) =>
{
	const single = Array.isArray(value) ? value[0] : value
	folder.value = single ?? ''
}

/**
 * Skipping is allowed on purpose: a player who has not been given the repository
 * yet should still reach the workspace, where both settings are editable inline.
 */
const finish = async () =>
{
	if (chosen.value === null) return

	if (isValidGithubRepo(repo.value))
	{
		appStore.githubRepo = repo.value.trim()
	}
	if (folder.value.trim().length > 0)
	{
		appStore.modpackPath = folder.value.trim()
	}

	appStore.setMode(chosen.value)
	await navigateTo('/dashboard')
}

// A returning user never sees this screen.
onMounted(async () =>
{
	if (appStore.modeChosen)
	{
		await navigateTo('/dashboard', { replace: true })
	}
})
</script>
