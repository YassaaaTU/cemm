<template>
  <!--
    The code is asked for on the card, not on the next screen, so choosing a
    pack and saying which update to put on it stay one gesture. This is also the
    last place before a fetch where the full path can be stated, which the card
    itself has no room for.
  -->
  <dialog
    ref="dialogRef"
    class="modal modal-bottom sm:modal-middle"
    aria-labelledby="pack-update-title"
    @cancel="handleCancel"
    @close="handleNativeClose"
  >
    <div class="modal-box max-w-md border border-base-300 bg-base-200 p-0">
      <div class="border-b border-base-300 px-4 py-3">
        <h3
          id="pack-update-title"
          class="truncate text-base font-bold"
        >
          Update {{ pack?.name ?? 'pack' }}
        </h3>
        <p
          class="mt-0.5 truncate font-mono text-[0.6875rem] text-base-content/55"
          :title="pack?.instancePath"
        >
          {{ pack?.instancePath }}
        </p>
      </div>

      <div class="space-y-3 p-4">
        <label
          class="block text-xs font-medium tracking-wide text-base-content/60 uppercase"
          for="pack-update-code"
        >
          Update code
        </label>
        <label class="input w-full border-base-300 bg-base-100 font-mono text-sm input-md">
          <Icon
            name="mdi:key-variant"
            size="1rem"
            class="shrink-0 text-base-content/40"
            aria-hidden="true"
          />
          <input
            id="pack-update-code"
            ref="codeRef"
            v-model="code"
            type="text"
            class="grow"
            placeholder="paste update code…"
            spellcheck="false"
            autocomplete="off"
            :disabled="busy"
            @keydown.enter="submit"
          />
        </label>

        <p class="text-sm/relaxed text-base-content/65">
          Nothing is written to this pack yet. You will see exactly what changes
          before anything is installed.
        </p>

        <div
          v-if="error !== null"
          role="alert"
          class="alert alert-soft text-sm alert-error"
        >
          <Icon
            name="mdi:alert-circle-outline"
            size="1.1rem"
            aria-hidden="true"
          />
          <span class="min-w-0 wrap-break-word">{{ error }}</span>
        </div>
      </div>

      <div class="modal-action mt-0 flex gap-2 border-t border-base-300 px-4 py-3">
        <button
          type="button"
          class="btn btn-ghost btn-sm"
          :disabled="busy"
          @click="dialogRef?.close()"
        >
          Cancel
        </button>
        <button
          type="button"
          class="btn gap-1.5 btn-primary btn-sm"
          :disabled="busy || code.trim().length === 0"
          @click="submit"
        >
          <span
            v-if="busy"
            class="loading loading-xs loading-spinner"
            aria-hidden="true"
          />
          {{ busy ? 'Fetching…' : 'Fetch update' }}
        </button>
      </div>
    </div>

    <form
      v-if="!busy"
      method="dialog"
      class="modal-backdrop"
    >
      <button>Close</button>
    </form>
  </dialog>
</template>

<script setup lang="ts">
import type { PackRow } from '~/stores/packs'

const props = defineProps<{
	pack: PackRow | null
	busy: boolean
	error: string | null
}>()

const emit = defineEmits<{ submit: [code: string], close: [] }>()

const dialogRef = ref<HTMLDialogElement | null>(null)
const codeRef = ref<HTMLInputElement | null>(null)
const code = ref('')

const submit = () =>
{
	if (props.busy || code.value.trim().length === 0) return
	emit('submit', code.value.trim())
}

/** A fetch in flight is not dismissable, for the same reason an install is not. */
const handleCancel = (event: Event) =>
{
	if (props.busy) event.preventDefault()
}

const handleNativeClose = () =>
{
	emit('close')
}

watch(() => props.pack, async (pack) =>
{
	const dialog = dialogRef.value
	if (dialog === null) return

	if (pack !== null)
	{
		// Each pack gets a fresh field: a code typed against one modpack is not a
		// code for a different one.
		code.value = ''
		if (!dialog.open) dialog.showModal()
		await nextTick()
		codeRef.value?.focus()
	}
	else if (dialog.open)
	{
		dialog.close()
	}
}, { immediate: true })
</script>
