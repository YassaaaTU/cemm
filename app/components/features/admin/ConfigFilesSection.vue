<template>
  <div class="card bg-base-200 shadow-lg">
    <div class="card-body">
      <h3 class="card-title text-lg">
        Config Files (Optional)
      </h3>
      <p class="text-sm opacity-70 mb-4">
        Select configuration files to include with your modpack update.
      </p>

      <!-- Config File Selection Buttons -->
      <div class="flex gap-2 mb-4">
        <button
          class="btn btn-outline btn-sm"
          @click="handleSelectFiles"
        >
          <Icon
            name="mdi:file-plus"
            size="1.2rem"
            class="mr-1"
          />
          Add Config Files
        </button>
        <button
          class="btn btn-outline btn-sm"
          @click="showDirectorySelector = true"
        >
          <Icon
            name="mdi:folder-plus"
            size="1.2rem"
            class="mr-1"
          />
          Add From Directory
        </button>
        <button
          v-if="modelValue.length > 0"
          class="btn btn-outline btn-error btn-sm"
          @click="clearFiles"
        >
          <Icon
            name="mdi:trash-can"
            size="1.2rem"
            class="mr-1"
          />
          Clear All
        </button>
      </div>

      <!-- Directory Selector Modal -->
      <dialog
        :class="['modal', { 'modal-open': showDirectorySelector }]"
      >
        <div class="modal-box w-11/12 max-w-2xl">
          <h3 class="font-bold text-lg mb-4">
            Select Config Directory
          </h3>
          <p class="text-sm opacity-70 mb-4">
            Choose a directory to scan for config files. All supported files will be added automatically.
          </p>

          <path-selector
            type="directory"
            title="Select Config Directory"
            @selected="handleDirectorySelected"
            @error="handleDirectoryError"
          />

          <div class="modal-action">
            <button
              class="btn btn-ghost"
              @click="showDirectorySelector = false"
            >
              Cancel
            </button>
          </div>
        </div>
        <div
          class="modal-backdrop"
          @click="showDirectorySelector = false"
        />
      </dialog>

      <!-- Selected Config Files List -->
      <div
        v-if="modelValue.length > 0"
        class="space-y-2"
      >
        <div class="text-sm font-medium opacity-80">
          Selected Files ({{ modelValue.length }}):
        </div>
        <div
          v-for="(configFile, index) in modelValue"
          :key="index"
          class="flex items-center justify-between p-3 bg-base-100 rounded-lg"
        >
          <div class="flex items-center gap-3">
            <div
              class="badge badge-sm"
              :class="configFile.is_binary ? 'badge-secondary' : 'badge-primary'"
            >
              {{ configFile.is_binary ? 'BINARY' : 'CONFIG' }}
            </div>
            <span class="font-mono text-sm">{{ configFile.relative_path }}</span>
            <span class="text-xs opacity-60">
              <template v-if="configFile.is_binary">
                Binary file ({{ configFile.filename.split('.').pop()?.toUpperCase() || 'BINARY' }})
              </template>
              <template v-else>
                ({{ Math.round(configFile.content.length / 1024 * 100) / 100 }} KB)
              </template>
            </span>
          </div>
          <button
            class="btn btn-ghost btn-xs btn-circle"
            @click="removeFile(configFile)"
          >
            <Icon
              name="mdi:close"
              size="1.2rem"
              class="text-error"
            />
          </button>
        </div>
      </div>

      <div
        v-else
        class="text-center py-6 opacity-60"
      >
        <Icon
          name="mdi:file-document-outline"
          size="4rem"
          class="text-gray-400 mb-2"
        />
        <p class="text-sm">
          No config files selected
        </p>
        <p class="text-xs opacity-60">
          Config files will be applied to the user's modpack directory
        </p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { ConfigFileWithContent } from '~/types'

const props = defineProps<{
	modelValue: ConfigFileWithContent[]
}>()

const emit = defineEmits<{
	'update:modelValue': [value: ConfigFileWithContent[]]
	'status': [message: string, type: 'success' | 'error' | 'info' | 'warning']
}>()

const showDirectorySelector = ref(false)

async function handleSelectFiles()
{
	const { selectConfigFiles } = useAdminApi()
	const newFiles = await selectConfigFiles((message: string, type: 'success' | 'error' | 'info' | 'warning') =>
	{
		emit('status', message, type)
	})
	if (newFiles.length > 0)
	{
		emit('update:modelValue', [...props.modelValue, ...newFiles])
	}
}

async function handleDirectorySelected(dirPath: string | string[])
{
	showDirectorySelector.value = false
	const pathToUse = Array.isArray(dirPath) ? dirPath[0] : dirPath

	if (pathToUse === undefined || pathToUse.trim().length === 0)
	{
		emit('status', 'No directory selected.', 'warning')
		return
	}

	const { scanDirectoryForConfigFiles } = useAdminApi()
	const newFiles = await scanDirectoryForConfigFiles(pathToUse, (message: string, type: 'success' | 'error' | 'info' | 'warning') =>
	{
		emit('status', message, type)
	})
	if (newFiles.length > 0)
	{
		emit('update:modelValue', [...props.modelValue, ...newFiles])
	}
}

function handleDirectoryError(error: string)
{
	showDirectorySelector.value = false
	emit('status', `Directory selection error: ${error}`, 'error')
}

function removeFile(configFile: ConfigFileWithContent)
{
	const index = props.modelValue.findIndex((cf) => cf.relative_path === configFile.relative_path)
	if (index !== -1)
	{
		const newValue = [...props.modelValue]
		newValue.splice(index, 1)
		emit('update:modelValue', newValue)
		emit('status', `Removed config file: ${configFile.relative_path}`, 'info')
	}
}

function clearFiles()
{
	emit('update:modelValue', [])
	emit('status', 'Cleared all config files.', 'info')
}
</script>
