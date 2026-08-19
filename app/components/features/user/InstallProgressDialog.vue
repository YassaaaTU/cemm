<template>
  <!--
    An install used to replace the whole screen with a progress bar: the diff
    the player had just read and consented to vanished the moment they acted on
    it, and came back as a bare "Update installed" panel with nothing to check
    it against. The work now happens in front of the diff rather than in place
    of it.

    Native <dialog> so focus trapping, Escape and the top layer come from the
    platform. Being modal also makes this the only surface reachable while an
    install runs — the rail could previously be clicked mid-install, unmounting
    the panel out from under work that carries on regardless in Rust.
  -->
  <dialog
    ref="dialogRef"
    class="modal modal-bottom sm:modal-middle"
    aria-labelledby="install-dialog-title"
    @cancel="handleCancel"
    @close="handleNativeClose"
  >
    <div class="modal-box max-w-md border border-base-300 bg-base-200 p-0">
      <div class="flex items-center gap-2.5 border-b border-base-300 px-4 py-3">
        <Icon
          :name="headerIcon"
          size="1.25rem"
          class="shrink-0"
          :class="headerTone"
          aria-hidden="true"
        />
        <h3
          id="install-dialog-title"
          class="min-w-0 text-base font-bold"
        >
          {{ headerTitle }}
        </h3>
      </div>

      <div class="space-y-3 p-4">
        <template v-if="state === 'running'">
          <div class="space-y-1.5">
            <div class="flex items-baseline justify-between gap-3">
              <p class="min-w-0 truncate text-sm font-medium">
                {{ label }}
              </p>
              <p class="shrink-0 font-mono text-xs text-base-content/60 tabular-nums">
                {{ Math.round(progress) }}%
              </p>
            </div>
            <progress
              class="progress w-full"
              :value="progress"
              max="100"
              :aria-label="label"
            />
          </div>
          <p class="text-sm/relaxed text-base-content/65">
            Files are being written to your modpack. Leave CEMM open until this
            finishes.
          </p>
        </template>

        <template v-else-if="state === 'done'">
          <div
            role="status"
            class="flex items-start gap-3 rounded-box border border-success/50 bg-success/10 px-3.5 py-3"
          >
            <Icon
              name="mdi:check-circle-outline"
              size="1.25rem"
              class="mt-px shrink-0 text-success"
              aria-hidden="true"
            />
            <div class="min-w-0">
              <p class="text-sm font-semibold text-success">
                Your modpack now matches the update
              </p>
              <p class="mt-0.5 text-xs text-base-content/65">
                You can launch the game.
              </p>
            </div>
          </div>

          <!-- What actually changed, in the same four words the diff behind this
               dialog uses, so closing it lands on a screen that agrees. -->
          <dl
            v-if="summary !== null"
            class="grid grid-cols-3 gap-2.5"
          >
            <div
              v-for="tally in tallies"
              :key="tally.label"
              class="rounded-box border border-base-300 bg-base-100 px-3 py-2"
            >
              <dt class="text-xs text-base-content/50">
                {{ tally.label }}
              </dt>
              <dd
                class="mt-1 font-mono text-lg leading-none font-bold tabular-nums"
                :class="tally.count > 0 ? tally.tone : 'text-base-content/25'"
              >
                {{ tally.count }}
              </dd>
            </div>
          </dl>
        </template>

        <template v-else>
          <div
            role="alert"
            class="alert alert-soft text-sm alert-error"
          >
            <Icon
              name="mdi:alert-circle-outline"
              size="1.1rem"
              aria-hidden="true"
            />
            <span class="min-w-0 wrap-break-word">
              {{ error !== null && error.length > 0 ? error : 'The install did not finish.' }}
            </span>
          </div>
          <!-- Deliberately does not claim the modpack is untouched: the install
               writes and deletes file by file, so a failure part-way through
               leaves whatever it had already done. Saying otherwise would be the
               one thing this screen must never do. -->
          <p class="text-sm/relaxed text-base-content/65">
            Files written before this point are still in place. The update you
            reviewed is still loaded — close this and run it again to finish.
          </p>
        </template>
      </div>

      <div class="modal-action mt-0 flex gap-2 border-t border-base-300 px-4 py-3">
        <button
          v-if="state === 'running'"
          type="button"
          class="btn btn-sm"
          disabled
        >
          <span
            class="loading loading-xs loading-spinner"
            aria-hidden="true"
          />
          Installing…
        </button>
        <button
          v-else
          ref="dismissRef"
          type="button"
          class="btn btn-sm"
          :class="state === 'done' ? 'btn-primary' : ''"
          @click="dialogRef?.close()"
        >
          {{ state === 'done' ? 'Done' : 'Close' }}
        </button>
      </div>
    </div>

    <!-- No backdrop dismissal while the install is in flight: closing mid-write
         would hide the only progress there is, and the work would carry on. -->
    <form
      v-if="state !== 'running'"
      method="dialog"
      class="modal-backdrop"
    >
      <button>Close</button>
    </form>
  </dialog>
</template>

<script setup lang="ts">
export interface InstallSummary
{
	added: number
	updated: number
	removed: number
}

const props = withDefaults(
	defineProps<{
		open: boolean
		state: 'running' | 'done' | 'failed'
		/** Already eased by the caller; this component does no smoothing. */
		progress: number
		label: string
		error?: string | null
		summary?: InstallSummary | null
	}>(),
	{ error: null, summary: null }
)

const emit = defineEmits<{ close: [] }>()

const dialogRef = ref<HTMLDialogElement | null>(null)
const dismissRef = ref<HTMLButtonElement | null>(null)

const headerTitle = computed(() =>
{
	if (props.state === 'done') return 'Update installed'
	if (props.state === 'failed') return 'Install failed'
	return 'Installing update'
})

const headerIcon = computed(() =>
{
	if (props.state === 'done') return 'mdi:check-circle-outline'
	if (props.state === 'failed') return 'mdi:alert-circle-outline'
	return 'mdi:package-down'
})

const headerTone = computed(() =>
{
	if (props.state === 'done') return 'text-success'
	if (props.state === 'failed') return 'text-error'
	return 'text-primary'
})

const tallies = computed(() =>
{
	const summary = props.summary
	if (summary === null) return []
	return [
		{ label: 'Added', count: summary.added, tone: 'text-success' },
		{ label: 'Updated', count: summary.updated, tone: 'text-info' },
		{ label: 'Deleted', count: summary.removed, tone: 'text-error' }
	]
})

/**
 * Escape is the platform's own dismissal, and it has to obey the same rule the
 * buttons do: an install in flight cannot be dismissed.
 */
const handleCancel = (event: Event) =>
{
	if (props.state === 'running') event.preventDefault()
}

const handleNativeClose = () =>
{
	emit('close')
}

watch(() => props.open, (open) =>
{
	const dialog = dialogRef.value
	if (dialog === null) return

	if (open)
	{
		if (!dialog.open) dialog.showModal()
	}
	else if (dialog.open)
	{
		dialog.close()
	}
}, { immediate: true })

/**
 * The dialog opens with no focusable control — the only button is disabled
 * while work runs — so focus is moved to the dismiss button the moment one
 * exists, rather than leaving the keyboard stranded on the dialog itself.
 */
watch(() => props.state, async (state) =>
{
	if (state === 'running') return
	await nextTick()
	dismissRef.value?.focus()
})
</script>
