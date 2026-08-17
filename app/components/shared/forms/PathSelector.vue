<template>
  <div class="w-full">
    <!-- One field, one button. The previous build made the user first choose
         between "Browse" and "Manual Input" modes before they could do
         anything; that is a decision about the app's plumbing, not about their
         modpack. Browsing is the primary path and typing works in the same
         field, validated as you go. -->
    <div class="flex items-stretch gap-2">
      <label
        class="input input-sm flex-1 border-base-300 bg-base-100 font-mono text-xs"
        :class="validationStateClass"
      >
        <Icon
          :name="type === 'directory' ? 'mdi:folder-outline' : 'mdi:file-outline'"
          size="0.9375rem"
          class="shrink-0 text-base-content/40"
          aria-hidden="true"
        />
        <input
          :id="inputId"
          v-model="draft"
          type="text"
          class="grow text-accent"
          :placeholder="inputPlaceholder"
          :aria-label="title"
          :aria-describedby="`${inputId}-status`"
          spellcheck="false"
          autocomplete="off"
        />
        <span
          v-if="validating"
          class="loading loading-spinner loading-xs shrink-0 text-base-content/40"
          aria-hidden="true"
        />
        <button
          v-else-if="draft.length > 0"
          type="button"
          class="btn btn-ghost btn-xs shrink-0 px-1"
          aria-label="Clear path"
          @click="clearSelection"
        >
          <Icon
            name="mdi:close"
            size="0.875rem"
            aria-hidden="true"
          />
        </button>
      </label>

      <button
        type="button"
        class="btn btn-sm shrink-0 gap-1.5 border-secondary/40 bg-base-200 hover:border-secondary hover:bg-base-300"
        :disabled="loading"
        @click="openDialog"
      >
        <span
          v-if="loading"
          class="loading loading-spinner loading-xs"
          aria-hidden="true"
        />
        <Icon
          v-else
          name="mdi:folder-search-outline"
          size="1rem"
          aria-hidden="true"
        />
        {{ loading ? 'Opening…' : 'Browse' }}
      </button>
    </div>

    <!-- Validation result. role="status" so the outcome of typing a path is
         announced rather than only appearing in colour. -->
    <p
      :id="`${inputId}-status`"
      class="mt-1.5 flex items-center gap-1.5 text-xs"
      :class="statusToneClass"
      role="status"
    >
      <Icon
        v-if="pathValidation !== null"
        :name="pathValidation.valid ? 'mdi:check-circle-outline' : 'mdi:alert-circle-outline'"
        size="0.875rem"
        class="shrink-0"
        aria-hidden="true"
      />
      {{ statusMessage }}
    </p>
  </div>
</template>

<script setup lang="ts">
interface Props
{
	type: 'directory' | 'file'
	title?: string
	multiple?: boolean
	modelValue?: string | string[]
	/** Copy shown before anything has been entered. */
	hint?: string
}

interface Emits
{
	(e: 'update:modelValue', value: string | string[] | null): void
	(e: 'selected', value: string | string[]): void
	(e: 'error', error: string): void
}

const props = withDefaults(defineProps<Props>(), {
	title: 'Select Path',
	multiple: false,
	hint: ''
})

const emit = defineEmits<Emits>()

const { selectDirectory, selectFile, selectMultipleFiles, validatePath } = useTauri()
const { $logger: logger } = useNuxtApp()

const inputId = useId()

const loading = ref(false)
const validating = ref(false)
/** What is in the field right now — typed or filled in by the dialog. */
const draft = ref('')
let validationGeneration = 0
const pathValidation = ref<{
	valid: boolean
	message: string
	details?: Record<string, unknown>
} | null>(null)

const inputPlaceholder = computed(() =>
	props.type === 'directory'
		? 'C:\\Users\\you\\curseforge\\minecraft\\Instances\\MyPack'
		: 'C:\\path\\to\\file.json'
)

const statusMessage = computed(() =>
{
	if (validating.value) return 'Checking…'
	if (pathValidation.value !== null) return pathValidation.value.message
	if (draft.value.length === 0 && props.hint.length > 0) return props.hint
	return ''
})

const statusToneClass = computed(() =>
{
	if (pathValidation.value === null) return 'text-base-content/55'
	return pathValidation.value.valid ? 'text-success' : 'text-error'
})

const validationStateClass = computed(() =>
{
	if (pathValidation.value === null) return ''
	return pathValidation.value.valid ? 'border-success/60' : 'border-error/60'
})

const openDialog = async () =>
{
	loading.value = true

	try
	{
		let result: string | string[] | null = null

		if (props.type === 'directory')
		{
			result = await selectDirectory()
		}
		else if (props.multiple)
		{
			result = await selectMultipleFiles()
		}
		else
		{
			result = await selectFile()
		}

		if (result !== null)
		{
			// A path from the native dialog is already known-good, so it commits
			// immediately without a validation round trip.
			draft.value = Array.isArray(result) ? result.join(', ') : result
			pathValidation.value = { valid: true, message: 'Selected.' }
			emit('update:modelValue', result)
			emit('selected', result)
		}
	}
	catch (error)
	{
		// selectDirectory/selectFile/selectMultipleFiles all catch their own
		// errors internally and resolve to null/[] rather than rejecting
		// (useTauri.ts), so this branch is unreachable today — kept only so a
		// future change to that error-swallowing doesn't produce an unhandled
		// rejection here.
		logger.error({ error }, 'PathSelector dialog error')
		const errorMessage = error instanceof Error ? error.message : String(error)
		emit('error', errorMessage)
	}
	finally
	{
		loading.value = false
	}
}

const validateTypedPath = async (path: string, generation: number) =>
{
	validating.value = true

	try
	{
		const result = await validatePath(path)
		if (generation !== validationGeneration || draft.value.trim() !== path)
		{
			return
		}

		if (!result.exists)
		{
			pathValidation.value = {
				valid: false,
				message: 'No folder or file at that path.',
				details: result
			}
			return
		}

		if (props.type === 'directory' && (result.is_directory ?? false))
		{
			pathValidation.value = {
				valid: true,
				message: (result.is_likely_modpack ?? false)
					? 'Looks like a modpack folder.'
					: 'Folder found — but no modpack files were detected inside it.',
				details: result
			}
		}
		else if (props.type === 'file' && (result.is_file ?? false))
		{
			pathValidation.value = {
				valid: true,
				message: (result.is_valid_config ?? false) ? 'Valid config file.' : 'File found.',
				details: result
			}
		}
		else
		{
			pathValidation.value = {
				valid: false,
				message: props.type === 'directory'
					? 'That is a file. Pick the folder that contains it.'
					: 'That is a folder. Pick a file inside it.',
				details: result
			}
			return
		}

		// Valid typed paths commit on their own. Requiring a separate confirm
		// click after typing a correct path is a step with no decision in it.
		commitTypedPath()
	}
	catch (error)
	{
		if (generation !== validationGeneration || draft.value.trim() !== path)
		{
			return
		}

		pathValidation.value = { valid: false, message: 'Could not check that path.' }
		logger.error({ error }, 'PathSelector path validation error')
	}
	finally
	{
		if (generation === validationGeneration && draft.value.trim() === path)
		{
			validating.value = false
		}
	}
}

const commitTypedPath = () =>
{
	const validation = pathValidation.value
	if (validation === null || !validation.valid || validation.details === undefined) return

	const details = validation.details
	const absolute = 'absolute_path' in details && typeof details.absolute_path === 'string'
		? details.absolute_path
		: draft.value.trim()

	emit('update:modelValue', absolute)
	emit('selected', absolute)
}

const debouncedValidate = useDebounceFn((path: string, generation: number) =>
{
	if (generation !== validationGeneration || draft.value.trim() !== path)
	{
		return
	}

	void validateTypedPath(path, generation)
}, 300)

watch(draft, (next) =>
{
	const path = next.trim()
	const generation = ++validationGeneration
	pathValidation.value = null

	if (path.length === 0)
	{
		validating.value = false
		return
	}

	debouncedValidate(path, generation)
})

const clearSelection = () =>
{
	draft.value = ''
	pathValidation.value = null
	validationGeneration++
	emit('update:modelValue', null)
}

watch(() => props.modelValue, (next) =>
{
	const incoming = Array.isArray(next) ? next.join(', ') : (next ?? '')
	// Only adopt an external value when it actually differs, so the parent
	// echoing the value back does not restart validation on every keystroke.
	if (incoming !== draft.value.trim())
	{
		draft.value = incoming
	}
}, { immediate: true })
</script>
