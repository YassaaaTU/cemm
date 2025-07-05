<template>
  <div class="w-full">
    <!-- Mode Toggle -->
    <div class="flex gap-2 mb-4">
      <button
        :class="[
          'btn btn-sm',
          useDialog ? 'btn-primary' : 'btn-outline',
        ]"
        @click="useDialog = true"
      >
        <Icon
          name="mdi:folder-open"
          size="1rem"
          class="mr-1"
        />
        Browse
      </button>
      <button
        :class="[
          'btn btn-sm',
          !useDialog ? 'btn-primary' : 'btn-outline',
        ]"
        @click="useDialog = false"
      >
        <Icon
          name="mdi:keyboard"
          size="1rem"
          class="mr-1"
        />
        Manual Input
      </button>
    </div>

    <!-- File Dialog Mode -->
    <div
      v-if="useDialog"
      class="space-y-4"
    >
      <button
        class="btn btn-primary w-full"
        :disabled="loading"
        @click="openDialog"
      >
        <loading-spinner
          v-if="loading"
          :loading="loading"
        />
        <Icon
          v-else
          name="mdi:folder-search"
          size="1.2rem"
          class="mr-2"
        />
        {{ dialogButtonText }}
      </button>

      <!-- Linux Fallback Message -->
      <div
        v-if="showLinuxFallback"
        class="alert alert-warning"
      >
        <Icon
          name="mdi:alert"
          size="1.2rem"
        />
        <div>
          <h4 class="font-bold">
            Dialog not responding?
          </h4>
          <p class="text-sm">
            If the file dialog freezes on Linux, try the "Manual Input" option above.
          </p>
        </div>
      </div>
    </div>

    <!-- Manual Input Mode -->
    <div
      v-else
      class="space-y-4"
    >
      <div class="form-control">
        <label class="label">
          <span class="label-text">{{ inputLabel }}</span>
        </label>
        <div class="relative">
          <input
            v-model="manualPath"
            type="text"
            class="input input-bordered w-full pr-10"
            :class="{
              'input-error': pathValidation && !pathValidation.valid,
              'input-success': pathValidation && pathValidation.valid,
            }"
            :placeholder="inputPlaceholder"
            @input="validateManualPath"
            @paste="validateManualPath"
          />
          <div class="absolute inset-y-0 right-0 flex items-center pr-3">
            <Icon
              v-if="validating"
              name="mdi:loading"
              size="1.2rem"
              class="animate-spin text-gray-400"
            />
            <Icon
              v-else-if="pathValidation && pathValidation.valid"
              name="mdi:check-circle"
              size="1.2rem"
              class="text-success"
            />
            <Icon
              v-else-if="pathValidation && !pathValidation.valid"
              name="mdi:alert-circle"
              size="1.2rem"
              class="text-error"
            />
          </div>
        </div>

        <!-- Validation feedback -->
        <div
          v-if="pathValidation"
          class="label"
        >
          <span
            class="label-text-alt"
            :class="{
              'text-success': pathValidation.valid,
              'text-error': !pathValidation.valid,
            }"
          >
            {{ pathValidation.message }}
          </span>
        </div>
      </div>

      <button
        class="btn btn-primary w-full"
        :disabled="!pathValidation?.valid || validating"
        @click="selectManualPath"
      >
        <Icon
          name="mdi:check"
          size="1.2rem"
          class="mr-2"
        />
        Use This Path
      </button>
    </div>

    <!-- Current Selection Display -->
    <div
      v-if="selectedPath"
      class="mt-4 p-3 bg-base-200 rounded-lg"
    >
      <div class="flex items-center gap-2">
        <Icon
          :name="type === 'directory' ? 'mdi:folder' : 'mdi:file'"
          size="1.2rem"
          class="text-primary"
        />
        <div class="flex-1 min-w-0">
          <div class="font-mono text-sm truncate">
            {{ selectedPath }}
          </div>
          <div class="text-xs opacity-60">
            Selected {{ type }}
          </div>
        </div>
        <button
          class="btn btn-ghost btn-xs"
          @click="clearSelection"
        >
          <Icon
            name="mdi:close"
            size="1rem"
          />
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
interface Props
{
	type: 'directory' | 'file'
	title?: string
	multiple?: boolean
	filters?: Array<{ name: string, extensions: string[] }>
	modelValue?: string | string[]
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
	filters: () => []
})

const emit = defineEmits<Emits>()

// Composables
const { selectDirectory, selectFile, selectMultipleFiles, validatePath } = useTauri()

// State
const useDialog = ref(true)
const loading = ref(false)
const validating = ref(false)
const manualPath = ref('')
const selectedPath = ref<string | null>(null)
const pathValidation = ref<{
	valid: boolean
	message: string
	details?: Record<string, unknown>
} | null>(null)
const showLinuxFallback = ref(false)

// Computed
const dialogButtonText = computed(() =>
{
	if (props.type === 'directory')
	{
		return loading.value ? 'Opening...' : 'Select Directory'
	}
	return loading.value ? 'Opening...' : props.multiple ? 'Select Files' : 'Select File'
})

const inputLabel = computed(() =>
{
	return props.type === 'directory' ? 'Directory Path' : 'File Path'
})

const inputPlaceholder = computed(() =>
{
	if (props.type === 'directory')
	{
		return '/home/user/minecraft/modpack or C:\\Users\\User\\curseforge\\minecraft\\Instances\\MyPack'
	}
	return '/path/to/file.json or C:\\path\\to\\file.json'
})

const _showSuggestions = computed(() => props.type === 'directory')

const _pathSuggestions = computed(() =>
{
	const suggestions = []

	// Detect OS and provide relevant suggestions
	if (typeof navigator !== 'undefined')
	{
		const userAgent = navigator.userAgent.toLowerCase()

		if (userAgent.includes('linux'))
		{
			suggestions.push(
				{ label: 'Home', path: '~/', icon: 'mdi:home' },
				{ label: 'Downloads', path: '~/Downloads', icon: 'mdi:download' },
				{ label: 'Documents', path: '~/Documents', icon: 'mdi:file-document' }
			)
		}
		else if (userAgent.includes('win'))
		{
			suggestions.push(
				{ label: 'Documents', path: 'C:\\Users\\%USERNAME%\\Documents', icon: 'mdi:file-document' },
				{ label: 'Downloads', path: 'C:\\Users\\%USERNAME%\\Downloads', icon: 'mdi:download' },
				{ label: 'CurseForge', path: 'C:\\Users\\%USERNAME%\\curseforge\\minecraft\\Instances', icon: 'mdi:minecraft' }
			)
		}
		else if (userAgent.includes('mac'))
		{
			suggestions.push(
				{ label: 'Home', path: '~/', icon: 'mdi:home' },
				{ label: 'Downloads', path: '~/Downloads', icon: 'mdi:download' },
				{ label: 'Documents', path: '~/Documents', icon: 'mdi:file-document' }
			)
		}
	}

	return suggestions
})

// Methods
const openDialog = async () =>
{
	loading.value = true
	showLinuxFallback.value = false

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
			if (Array.isArray(result))
			{
				selectedPath.value = result.join(', ')
				emit('update:modelValue', result)
				emit('selected', result)
			}
			else
			{
				selectedPath.value = result
				emit('update:modelValue', result)
				emit('selected', result)
			}
		}
	}
	catch (error)
	{
		console.error('Dialog error:', error)

		// Check if it's a timeout or Linux-specific error
		const errorMessage = error instanceof Error ? error.message : String(error)
		if (errorMessage.includes('timeout') || errorMessage.includes('Dialog error'))
		{
			showLinuxFallback.value = true
			useDialog.value = false // Auto-switch to manual input
		}

		emit('error', errorMessage)
	}
	finally
	{
		loading.value = false
	}
}

const validateManualPath = async () =>
{
	if (!manualPath.value.trim())
	{
		pathValidation.value = null
		return
	}

	validating.value = true

	try
	{
		const result = await validatePath(manualPath.value.trim())

		if (result.exists)
		{
			if (props.type === 'directory' && (result.is_directory ?? false))
			{
				pathValidation.value = {
					valid: true,
					message: (result.is_likely_modpack ?? false)
						? '✓ Valid modpack directory'
						: '✓ Valid directory',
					details: result
				}
			}
			else if (props.type === 'file' && (result.is_file ?? false))
			{
				pathValidation.value = {
					valid: true,
					message: (result.is_valid_config ?? false)
						? '✓ Valid config file'
						: '✓ Valid file',
					details: result
				}
			}
			else
			{
				pathValidation.value = {
					valid: false,
					message: `❌ Expected a ${props.type}, but this is a ${(result.is_directory ?? false) ? 'directory' : 'file'}`,
					details: result
				}
			}
		}
		else
		{
			pathValidation.value = {
				valid: false,
				message: '❌ Path does not exist',
				details: result
			}
		}
	}
	catch (error)
	{
		pathValidation.value = {
			valid: false,
			message: '❌ Unable to validate path'
		}
		console.error('Path validation error:', error)
	}
	finally
	{
		validating.value = false
	}
}

const selectManualPath = () =>
{
	if (pathValidation.value !== null && pathValidation.value.valid && pathValidation.value.details !== undefined)
	{
		const details = pathValidation.value.details
		if ('absolute_path' in details && typeof details.absolute_path === 'string')
		{
			const absolutePath = details.absolute_path
			selectedPath.value = absolutePath
			emit('update:modelValue', absolutePath)
			emit('selected', absolutePath)
		}
	}
}

const clearSelection = () =>
{
	selectedPath.value = null
	manualPath.value = ''
	pathValidation.value = null
	emit('update:modelValue', null)
}

// Watch for external value changes
watch(() => props.modelValue, (newValue) =>
{
	if (typeof newValue === 'string')
	{
		selectedPath.value = newValue
	}
	else if (Array.isArray(newValue))
	{
		selectedPath.value = newValue.join(', ')
	}
	else
	{
		selectedPath.value = null
	}
}, { immediate: true })
</script>

<style scoped>
</style>
