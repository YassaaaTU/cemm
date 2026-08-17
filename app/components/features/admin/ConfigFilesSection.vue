<template>
  <div class="overflow-hidden rounded-box border border-base-300 bg-base-200">
    <div class="flex flex-wrap items-center gap-x-3 gap-y-2 border-b border-base-300 px-3 py-2.5">
      <h3
        id="config-files-title"
        class="text-sm font-semibold"
      >
        Config files
      </h3>

      <span class="font-mono text-xs tabular-nums text-base-content/50">{{ modelValue.length }}</span>

      <span class="flex-1" />

      <button
        type="button"
        class="btn btn-xs gap-1.5 border-base-300"
        :disabled="busy"
        @click="handleSelectFiles"
      >
        <Icon
          name="mdi:file-plus-outline"
          size="0.875rem"
          aria-hidden="true"
        />
        Add files
      </button>

      <!-- Scanning a folder used to go through a modal wrapping a path field.
           The native directory dialog is the same decision in one click. -->
      <button
        type="button"
        class="btn btn-xs gap-1.5 border-base-300"
        :disabled="busy"
        @click="handleScanDirectory"
      >
        <span
          v-if="busy"
          class="loading loading-spinner loading-xs"
          aria-hidden="true"
        />
        <Icon
          v-else
          name="mdi:folder-search-outline"
          size="0.875rem"
          aria-hidden="true"
        />
        Scan a folder
      </button>

      <button
        v-if="modelValue.length > 0"
        type="button"
        class="btn btn-ghost btn-xs gap-1.5 text-error"
        @click="clearFiles"
      >
        <Icon
          name="mdi:close"
          size="0.875rem"
          aria-hidden="true"
        />
        Clear
      </button>
    </div>

    <p
      v-if="modelValue.length === 0"
      class="px-3 py-5 text-center text-sm text-base-content/50"
    >
      No config files attached. Anything you add here is copied into every
      player's modpack, overwriting their version of that file.
    </p>

    <ul
      v-else
      class="max-h-52 overflow-y-auto"
      aria-labelledby="config-files-title"
    >
      <li
        v-for="file in modelValue"
        :key="file.relative_path"
        class="flex items-center gap-2 border-b border-base-300/40 px-3 py-1.5 last:border-b-0"
      >
        <StatusChip
          tone="unchanged"
          :label="file.is_binary === true ? 'BIN' : 'CFG'"
        />
        <span
          class="min-w-0 flex-1 truncate font-mono text-xs text-base-content/80"
          :title="file.relative_path"
        >{{ file.relative_path }}</span>
        <button
          type="button"
          class="btn btn-ghost btn-xs px-1 text-base-content/50 hover:text-error"
          :aria-label="`Remove ${file.relative_path}`"
          @click="removeFile(file)"
        >
          <Icon
            name="mdi:close"
            size="0.875rem"
            aria-hidden="true"
          />
        </button>
      </li>
    </ul>
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

const busy = ref(false)

const reportStatus = (message: string, type: 'success' | 'error' | 'info' | 'warning') =>
{
	emit('status', message, type)
}

/**
 * Config files are keyed by relative_path on the Rust side, so adding the same
 * path twice would ship a duplicate entry. Merging on that key keeps repeated
 * scans of overlapping folders idempotent.
 */
const mergeFiles = (incoming: ConfigFileWithContent[]) =>
{
	if (incoming.length === 0) return

	const byPath = new Map(props.modelValue.map((file) => [file.relative_path, file]))
	for (const file of incoming)
	{
		byPath.set(file.relative_path, file)
	}
	emit('update:modelValue', [...byPath.values()])
}

async function handleSelectFiles()
{
	busy.value = true
	try
	{
		const { selectConfigFiles } = useAdminApi()
		mergeFiles(await selectConfigFiles(reportStatus))
	}
	finally
	{
		busy.value = false
	}
}

async function handleScanDirectory()
{
	busy.value = true
	try
	{
		const { selectDirectory } = useTauri()
		const directory = await selectDirectory()

		if (directory === null || directory.trim().length === 0)
		{
			return
		}

		const { scanDirectoryForConfigFiles } = useAdminApi()
		mergeFiles(await scanDirectoryForConfigFiles(directory, reportStatus))
	}
	finally
	{
		busy.value = false
	}
}

function removeFile(configFile: ConfigFileWithContent)
{
	const next = props.modelValue.filter((file) => file.relative_path !== configFile.relative_path)
	if (next.length !== props.modelValue.length)
	{
		emit('update:modelValue', next)
		emit('status', `Removed ${configFile.relative_path}`, 'info')
	}
}

function clearFiles()
{
	emit('update:modelValue', [])
	emit('status', 'Cleared all config files.', 'info')
}
</script>
