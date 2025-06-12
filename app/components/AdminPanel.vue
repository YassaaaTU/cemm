<template>
  <div>
    <h2 class="text-2xl font-bold mb-4">
      Admin Mode
    </h2>
    <div class="flex gap-2 mt-4">
      <button
        class="btn btn-primary"
        @click="loadInstance"
      >
        Load Instance
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
import ManifestPreview from '~/components/ManifestPreview.vue'
import ProgressBar from '~/components/ProgressBar.vue'
import StatusAlert from '~/components/StatusAlert.vue'
import { useTauri } from '~/composables/useTauri'
import { useManifestStore } from '~/stores/manifest'

const progress = ref(0)
const statusMessage = ref('')
const statusType = ref<'success' | 'error' | 'info' | 'warning'>('info')

const { selectFile, selectSaveFile, writeFile, parseMinecraftInstance, compareManifests, readFile } = useTauri()
const manifestStore = useManifestStore()
const manifest = computed(() => manifestStore.manifest)

// Load a minecraftinstance.json and convert to manifest
async function loadInstance()
{
	statusMessage.value = ''
	// Select minecraftinstance.json
	const filePath = await selectFile()
	if (filePath == null || filePath.length === 0)
	{
		statusMessage.value = 'No file selected.'
		statusType.value = 'warning'
		return
	}
	// Parse minecraftinstance.json to Manifest
	const parsed = await parseMinecraftInstance(filePath)
	if (parsed == null)
	{
		statusMessage.value = 'Failed to parse minecraftinstance.json.'
		statusType.value = 'error'
		return
	}
	// Save previous manifest for diffing (store in Pinia)
	if (manifest.value != null)
	{
		manifestStore.setPreviousManifest(manifest.value)
	}
	manifestStore.setManifest(parsed)
	statusMessage.value = 'Manifest generated from minecraftinstance.json.'
	statusType.value = 'success'
	// If previous manifest exists, show diff (store in Pinia)
	if (manifestStore.previousManifest != null)
	{
		const diff = await compareManifests(manifestStore.previousManifest, parsed)
		manifestStore.setUpdateInfo(diff)
	}
	else
	{
		manifestStore.setUpdateInfo(null)
	}
}

// Save/export the generated manifest
async function saveManifest()
{
	statusMessage.value = ''
	if (manifest.value == null)
	{
		return
	}
	// Prompt user for save location (Save As dialog)
	const filePath = await selectSaveFile()
	if (filePath == null || filePath.length === 0)
	{
		statusMessage.value = 'No file selected.'
		statusType.value = 'warning'
		return
	}
	// Check if file exists (optional, for user feedback)
	let fileExists = false
	try
	{
		const existing = await readFile(filePath)
		if (typeof existing === 'string' && existing.length > 0)
		{
			fileExists = true
		}
	}
	catch (_err)
	{
		// File does not exist, proceed
	}
	if (fileExists)
	{
		statusMessage.value = 'File already exists. Overwriting.'
		statusType.value = 'warning'
	}
	const ok = await writeFile(filePath, JSON.stringify(manifest.value, null, 2))
	if (ok)
	{
		statusMessage.value = `Manifest saved as ${filePath}.`
		statusType.value = 'success'
	}
	else
	{
		statusMessage.value = 'Failed to save manifest.'
		statusType.value = 'error'
	}
}
</script>
