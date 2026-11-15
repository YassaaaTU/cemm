<template>
  <div class="min-h-0 flex-1 space-y-4 overflow-y-auto pr-1">
    <!-- What cemm-pack.json says this pack is. Stated first, because the
         release's identity comes from here and not from the profile's name. -->
    <div class="rounded-box border border-base-300 bg-base-200 px-4 py-3.5">
      <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
        <h3 class="text-[0.9375rem] font-semibold">
          {{ spec.name }}
        </h3>
        <span class="font-mono text-xs text-base-content/60">{{ spec.lineage }}</span>
      </div>
      <p class="mt-1 text-xs/relaxed text-base-content/65">
        Built on {{ spec.base.name.length > 0 ? spec.base.name : `official file ${spec.base.file_id}` }}
        · {{ (spec.content ?? []).length }} content {{ (spec.content ?? []).length === 1 ? 'path' : 'paths' }}
        · {{ (spec.pins ?? []).length }} {{ (spec.pins ?? []).length === 1 ? 'pin' : 'pins' }}
        · {{ (spec.settings ?? []).length }} required {{ (spec.settings ?? []).length === 1 ? 'setting' : 'settings' }}.
        Edit <span class="font-mono">cemm-pack.json</span> in the profile to change any of this.
      </p>
    </div>

    <div class="rounded-box border border-base-300 bg-base-200 px-4 py-3.5">
      <div class="grid gap-3 sm:grid-cols-2">
        <label class="block">
          <span class="mb-1 block text-xs font-medium tracking-wide text-base-content/60 uppercase">Release id</span>
          <input
            v-model="releaseId"
            type="text"
            class="input w-full border-base-300 bg-base-100 font-mono text-xs input-sm"
            placeholder="2026.11.15-1"
            spellcheck="false"
            autocomplete="off"
            :disabled="busy"
          />
          <span class="mt-1 block text-xs text-base-content/50">Becomes the second half of the code. Never reused.</span>
        </label>
        <label class="block">
          <span class="mb-1 block text-xs font-medium tracking-wide text-base-content/60 uppercase">Previous release code</span>
          <input
            v-model="previousCode"
            type="text"
            class="input w-full border-base-300 bg-base-100 font-mono text-xs input-sm"
            placeholder="leave empty for the first release"
            spellcheck="false"
            autocomplete="off"
            :disabled="busy"
          />
          <span class="mt-1 block text-xs text-base-content/50">Lets players who skipped it still be cleaned up correctly.</span>
        </label>
      </div>
      <label class="mt-3 block">
        <span class="mb-1 block text-xs font-medium tracking-wide text-base-content/60 uppercase">Release notes</span>
        <textarea
          v-model="notes"
          class="textarea w-full border-base-300 bg-base-100 text-sm textarea-sm"
          rows="3"
          placeholder="What changed, in a few lines your players will read."
          :disabled="busy"
        />
      </label>

      <div class="mt-3 flex flex-wrap items-center gap-2">
        <button
          type="button"
          class="btn gap-1.5 btn-sm"
          :disabled="busy || releaseId.trim().length === 0"
          @click="handleCheck"
        >
          <span
            v-if="checking"
            class="loading loading-xs loading-spinner"
            aria-hidden="true"
          />
          {{ checking ? 'Checking every file…' : 'Check release' }}
        </button>
        <button
          type="button"
          class="btn gap-1.5 btn-primary btn-sm"
          :disabled="!canPublish"
          @click="handlePublish"
        >
          <span
            v-if="publishing"
            class="loading loading-xs loading-spinner"
            aria-hidden="true"
          />
          Publish release
        </button>
        <span
          v-if="publishing"
          class="min-w-0 flex-1 truncate text-xs text-base-content/60"
        >{{ Math.round(progress) }}% · {{ progressMessage }}</span>
        <span
          v-else-if="draft !== null && !draftIsCurrent && receipt === null"
          class="text-xs text-warning"
        >Inputs changed since the check. Check again before publishing.</span>
      </div>
    </div>

    <!-- The receipt. The code is the deliverable; the rest is for the admin's
         own records, kept outside the profile. -->
    <div
      v-if="receipt !== null"
      class="rounded-box border border-success/50 bg-success/10 px-4 py-3.5"
    >
      <p class="text-sm font-semibold text-success">
        Published. Share this code:
      </p>
      <div class="mt-2 flex flex-wrap items-center gap-2">
        <code class="min-w-0 flex-1 rounded-field border border-base-300 bg-base-100 px-2.5 py-1.5 font-mono text-xs break-all [user-select:all]">{{ receipt.code }}</code>
        <button
          type="button"
          class="btn shrink-0 btn-sm"
          @click="copyCode"
        >
          {{ copied ? 'Copied' : 'Copy code' }}
        </button>
        <button
          type="button"
          class="btn shrink-0 btn-sm"
          @click="saveReceipt"
        >
          Save receipt
        </button>
      </div>
      <p class="mt-2 text-xs/relaxed text-base-content/65">
        {{ receipt.uploaded_objects }} new {{ receipt.uploaded_objects === 1 ? 'file' : 'files' }} uploaded,
        {{ receipt.reused_objects }} already in the repository.
        The part after <span class="font-mono">@</span> pins the code to this exact publication.
      </p>
    </div>

    <template v-if="draft !== null">
      <div
        v-if="draft.errors.length > 0"
        class="overflow-hidden rounded-box border border-error/60"
      >
        <p class="bg-error/10 px-3.5 py-2.5 text-sm font-semibold text-error">
          {{ draft.errors.length }} {{ draft.errors.length === 1 ? 'problem blocks' : 'problems block' }} this release
        </p>
        <ul>
          <li
            v-for="error in draft.errors"
            :key="error"
            class="border-t border-base-300/60 px-3.5 py-2 text-sm/relaxed"
          >
            {{ error }}
          </li>
        </ul>
      </div>
      <p
        v-else
        class="rounded-box border border-success/40 bg-success/10 px-3.5 py-2.5 text-sm"
      >
        Nothing blocks this release. It describes {{ draft.summary.addons }} addons and {{ draft.summary.files }} pack files.
      </p>

      <div class="grid grid-cols-2 gap-2.5 sm:grid-cols-4">
        <div
          v-for="tile in tiles"
          :key="tile.label"
          class="rounded-box border border-base-300 bg-base-200 px-3.5 py-3"
        >
          <p class="text-xs text-base-content/50">
            {{ tile.label }}
          </p>
          <p class="mt-1.5 font-mono text-xl leading-none font-bold tabular-nums">
            {{ tile.value }}
          </p>
        </div>
      </div>

      <div
        v-if="changeLines.length > 0"
        class="rounded-box border border-base-300 bg-base-200 px-3.5 py-3"
      >
        <h3 class="text-sm font-semibold">
          Since the previous release
        </h3>
        <ul class="mt-1.5 space-y-0.5 text-xs/relaxed text-base-content/75">
          <li
            v-for="line in changeLines"
            :key="line"
          >
            {{ line }}
          </li>
        </ul>
      </div>

      <div
        v-if="draft.warnings.length > 0"
        class="rounded-box border border-warning/50 bg-warning/5 px-3.5 py-3"
      >
        <h3 class="text-sm font-semibold">
          Worth a look
        </h3>
        <ul class="mt-1.5 list-disc space-y-1 pl-4 text-xs/relaxed text-base-content/75">
          <li
            v-for="warning in draft.warnings"
            :key="warning"
          >
            {{ warning }}
          </li>
        </ul>
      </div>
    </template>

    <!-- The three-way review for an official base upgrade. It reports; the
         admin builds the candidate by hand in a copy, then checks it above. -->
    <details class="rounded-box border border-base-300 bg-base-200">
      <summary class="cursor-pointer px-4 py-3 text-sm font-semibold">
        Review an official pack upgrade
      </summary>
      <div class="space-y-3 border-t border-base-300 px-4 py-3.5">
        <p class="text-xs/relaxed text-base-content/65">
          Compares the official release this pack was built on, this pack, and the new official
          release. It says which side to take for every file and mod, and lists what only you can
          decide. It changes nothing.
        </p>
        <PathSelector
          type="directory"
          title="Previous official release"
          hint="The installed official profile, or its extracted archive, that this pack was built on."
          :model-value="basePath"
          @update:model-value="(value) => basePath = single(value)"
        />
        <PathSelector
          type="directory"
          title="New official release"
          hint="A clean install or extracted archive of the new official release."
          :model-value="upstreamPath"
          @update:model-value="(value) => upstreamPath = single(value)"
        />
        <div class="flex flex-wrap items-center gap-2">
          <button
            type="button"
            class="btn gap-1.5 btn-sm"
            :disabled="busy || basePath.length === 0 || upstreamPath.length === 0"
            @click="handleReview"
          >
            <span
              v-if="reviewing"
              class="loading loading-xs loading-spinner"
              aria-hidden="true"
            />
            Compare
          </button>
          <button
            v-if="review !== null"
            type="button"
            class="btn btn-sm"
            @click="saveReview"
          >
            Save report
          </button>
        </div>

        <template v-if="review !== null">
          <p class="text-sm">
            <strong :class="review.conflicts > 0 ? 'text-error' : 'text-success'">
              {{ review.conflicts === 0 ? 'No conflicts.' : `${review.conflicts} ${review.conflicts === 1 ? 'conflict needs' : 'conflicts need'} your decision.` }}
            </strong>
            Take the official change for {{ review.take_upstream }},
            keep this pack's for {{ review.keep_custom }},
            {{ review.same_result }} already agree, {{ review.unchanged }} unchanged.
          </p>
          <ul
            v-if="conflictItems.length > 0"
            class="max-h-56 overflow-y-auto rounded-field border border-error/40"
          >
            <li
              v-for="item in conflictItems"
              :key="item.kind + item.key"
              class="border-b border-base-300/50 px-3 py-2 last:border-b-0"
            >
              <span class="block font-mono text-xs break-all">{{ item.label }}</span>
              <span class="mt-0.5 block text-xs text-base-content/65">{{ item.reason }}</span>
            </li>
          </ul>
          <ul
            v-if="review.review.length > 0"
            class="list-disc space-y-1 pl-4 text-xs/relaxed text-base-content/75"
          >
            <li
              v-for="note in review.review"
              :key="note"
            >
              {{ note }}
            </li>
          </ul>
        </template>
      </div>
    </details>
  </div>
</template>

<script setup lang="ts">
import type { PublishReceipt, ReleaseDraft, ReleaseSpec, ThreeWayReport } from '~/types'

const props = defineProps<{
	/** The profile folder holding `cemm-pack.json`. */
	instanceDir: string
	spec: ReleaseSpec
}>()

const emit = defineEmits<{
	status: [message: string, type: 'success' | 'error' | 'info' | 'warning']
	published: [receipt: PublishReceipt]
}>()

const { checkRelease, publishDraft, reviewOfficialUpgrade } = useReleaseAdmin()
const { selectSaveFile, writeFile } = useTauri()

const setStatus = (message: string, type: 'success' | 'error' | 'info' | 'warning') =>
{
	emit('status', message, type)
}

/**
 * The last code published for this pack, so the next release starts from it.
 * Per lineage, and only a convenience: the field is editable, and the release
 * it names is fetched from the repository before it is trusted for anything.
 */
const lastCodeKey = computed(() => `cemm-release-last:${props.spec.lineage}`)
const readLastCode = (): string =>
{
	try
	{
		return localStorage.getItem(lastCodeKey.value) ?? ''
	}
	catch
	{
		return ''
	}
}

const releaseId = ref('')
const previousCode = ref(readLastCode())
const notes = ref('')

const checking = ref(false)
const publishing = ref(false)
const reviewing = ref(false)
const progress = ref(0)
const progressMessage = ref('')
const draft = ref<ReleaseDraft | null>(null)
/** The inputs `draft` was checked with. A draft is only publishable against them. */
const checkedInputs = ref('')
const receipt = ref<PublishReceipt | null>(null)
const copied = ref(false)

const basePath = ref('')
const upstreamPath = ref('')
const review = ref<ThreeWayReport | null>(null)

const busy = computed(() => checking.value || publishing.value || reviewing.value)
const inputs = computed(() => JSON.stringify([props.instanceDir, releaseId.value.trim(), previousCode.value.trim(), notes.value]))
const draftIsCurrent = computed(() => draft.value !== null && checkedInputs.value === inputs.value)

const canPublish = computed(() =>
	!busy.value
	&& draftIsCurrent.value
	&& draft.value !== null
	&& draft.value.errors.length === 0
	&& receipt.value === null
)

const single = (value: string | string[] | null): string =>
	Array.isArray(value) ? (value[0] ?? '') : (value ?? '')

const megabytes = (bytes: number): string =>
	bytes >= 1024 * 1024 ? `${(bytes / 1024 / 1024).toFixed(1)} MB` : `${Math.max(1, Math.round(bytes / 1024))} KB`

const tiles = computed(() =>
{
	const summary = draft.value?.summary
	if (summary === undefined) return []
	return [
		{ label: 'Addons', value: `${summary.addons}` },
		{ label: 'Pack files', value: `${summary.files}` },
		{ label: 'To upload', value: summary.new_payloads === 0 ? '0' : `${summary.new_payloads} · ${megabytes(summary.new_payload_bytes)}` },
		{ label: 'Retired', value: `${summary.retired}` }
	]
})

const changeLines = computed(() =>
{
	const summary = draft.value?.summary
	if (summary === undefined) return []
	const lines = [
		...summary.addons_added.map((name) => `Added: ${name}`),
		...summary.addons_updated.map((name) => `Updated: ${name}`),
		...summary.addons_removed.map((name) => `Removed: ${name}`)
	]
	const files = [
		summary.files_added > 0 ? `${summary.files_added} added` : '',
		summary.files_changed > 0 ? `${summary.files_changed} changed` : '',
		summary.files_removed > 0 ? `${summary.files_removed} removed` : ''
	].filter((part) => part.length > 0)
	if (files.length > 0) lines.push(`Pack files: ${files.join(', ')}`)
	return lines
})

const conflictItems = computed(() =>
	(review.value?.items ?? []).filter((item) => item.decision === 'conflict')
)

async function handleCheck()
{
	checking.value = true
	receipt.value = null
	try
	{
		const checkedWith = inputs.value
		const result = await checkRelease(
			props.instanceDir,
			releaseId.value,
			notes.value,
			previousCode.value,
			setStatus
		)
		draft.value = result
		checkedInputs.value = result === null ? '' : checkedWith
	}
	finally
	{
		checking.value = false
	}
}

async function handlePublish()
{
	const current = draft.value
	if (!canPublish.value || current === null) return

	publishing.value = true
	progress.value = 0
	progressMessage.value = ''
	try
	{
		const published = await publishDraft(
			props.instanceDir,
			current.release,
			(value, message) =>
			{
				progress.value = value
				if (message !== undefined) progressMessage.value = message
			},
			setStatus
		)
		if (published === null) return

		receipt.value = published
		// The next release follows this one.
		previousCode.value = published.code
		try
		{
			localStorage.setItem(lastCodeKey.value, published.code)
		}
		catch
		{
			// Only the convenience is lost; the receipt still carries the code.
		}
		setStatus('Release published. Share the code with your players.', 'success')
		emit('published', published)
	}
	finally
	{
		publishing.value = false
	}
}

async function copyCode()
{
	if (receipt.value === null) return
	try
	{
		await navigator.clipboard.writeText(receipt.value.code)
		copied.value = true
		setTimeout(() =>
		{
			copied.value = false
		}, 2000)
	}
	catch
	{
		setStatus('Could not copy automatically. Select the code and copy it by hand.', 'warning')
	}
}

async function saveJson(value: unknown, what: string)
{
	const path = await selectSaveFile()
	if (path === null || path.length === 0) return
	const saved = await writeFile(path, JSON.stringify(value, null, 2))
	setStatus(saved ? `${what} saved to ${path}.` : `Could not save the ${what.toLowerCase()}.`, saved ? 'success' : 'error')
}

const saveReceipt = () => saveJson(receipt.value, 'Receipt')
const saveReview = () => saveJson(review.value, 'Upgrade report')

async function handleReview()
{
	reviewing.value = true
	try
	{
		review.value = await reviewOfficialUpgrade(
			basePath.value,
			props.instanceDir,
			upstreamPath.value,
			setStatus
		)
	}
	finally
	{
		reviewing.value = false
	}
}

// A different pack is a different release line: nothing checked carries over.
watch(() => props.instanceDir, () =>
{
	draft.value = null
	receipt.value = null
	review.value = null
	checkedInputs.value = ''
	previousCode.value = readLastCode()
})
</script>
