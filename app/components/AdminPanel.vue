<template>
  <div>
    <h2 class="text-2xl font-bold mb-4">
      Admin Mode
    </h2>
    <file-selector />
    <div class="flex gap-2 mt-4">
      <button
        class="btn btn-primary"
        @click="openManifest"
      >
        Open Manifest
      </button>
      <button
        class="btn btn-secondary"
        :disabled="manifest == null"
        @click="saveManifest"
      >
        Save Manifest
      </button>
    </div>
    <addon-list class="mt-4" />
    <manifest-preview class="mt-4" />
    <git-hub-settings class="mt-4" />
    <div class="mt-6 flex flex-col gap-2">
      <progress-bar :progress="progress" />
      <status-alert
        :message="statusMessage"
        :type="statusType"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import AddonList from '~/components/AddonList.vue'
import FileSelector from '~/components/FileSelector.vue'
import GitHubSettings from '~/components/GitHubSettings.vue'
import ManifestPreview from '~/components/ManifestPreview.vue'
import ProgressBar from '~/components/ProgressBar.vue'
import StatusAlert from '~/components/StatusAlert.vue'
import { useTauri } from '~/composables/useTauri'
import { useManifestStore } from '~/stores/manifest'

const progress = ref(0)
const statusMessage = ref('')
const statusType = ref<'success' | 'error' | 'info' | 'warning'>('info')

const { selectFile, writeFile, readFile } = useTauri()
const manifestStore = useManifestStore()
const manifest = computed(() => manifestStore.manifest)

async function openManifest()
{
	statusMessage.value = ''
	const filePath = await selectFile()
	if (filePath == null || filePath.length === 0)
	{
		statusMessage.value = 'No file selected.'
		statusType.value = 'warning'
		return
	}
	const content = await readFile(filePath)
	if (content == null || content.length === 0)
	{
		statusMessage.value = 'Failed to read file.'
		statusType.value = 'error'
		return
	}
	try
	{
		const parsed = JSON.parse(content)
		manifestStore.setManifest(parsed)
		statusMessage.value = 'Manifest loaded.'
		statusType.value = 'success'
	}
	catch (_err)
	{
		statusMessage.value = 'Invalid manifest format.'
		statusType.value = 'error'
	}
}

async function saveManifest()
{
	statusMessage.value = ''
	if (manifest.value == null)
	{
		return
	}
	const filePath = await selectFile()
	if (filePath == null || filePath.length === 0)
	{
		statusMessage.value = 'No file selected.'
		statusType.value = 'warning'
		return
	}
	const ok = await writeFile(filePath, JSON.stringify(manifest.value, null, 2))
	if (ok)
	{
		statusMessage.value = 'Manifest saved.'
		statusType.value = 'success'
	}
	else
	{
		statusMessage.value = 'Failed to save file.'
		statusType.value = 'error'
	}
}
</script>
