<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, ref } from 'vue'

import { useManifestStore } from '~/stores/manifest'
import type { Addon, Manifest, UpdateInfo } from '~/types'

const logger = usePinoLogger()

const props = defineProps<{ manifest?: Manifest | null, updateInfo?: UpdateInfo | null }>()

const manifestStore = useManifestStore()
const manifest = computed(() => props.manifest ?? manifestStore.manifest)
const updateInfo = computed(() => props.updateInfo ?? manifestStore.updateInfo)

const tab = ref<'mods' | 'resourcepacks' | 'shaderpacks' | 'datapacks'>('mods')

function getCategoryAddons(category: 'mods' | 'resourcepacks' | 'shaderpacks' | 'datapacks')
{
	if (manifest.value === null) return []
	return manifest.value[category]
}

function getAddonStatus(addon: Addon)
{
	if (updateInfo.value === null) return ''
	const added = updateInfo.value.addedAddons.find(
		(a) => a.addon_name === addon.addon_name && a.version === addon.version
	)
	const removed = updateInfo.value.removedAddons.includes(addon.addon_name)
	if (added != null) return 'added'
	if (removed) return 'removed'
	return ''
}

async function openCurseforge(addon: Addon)
{
	logger.info('Opening CurseForge page for addon', {
		addonName: addon.addon_name,
		webSiteURL: addon.webSiteURL,
		hasWebSiteURL: !(addon.webSiteURL == null)
	})

	if ((addon.webSiteURL != null) && addon.webSiteURL.length > 0)
	{
		logger.info('Attempting to open URL:', addon.webSiteURL)
		try
		{
			await invoke('open_url', { url: addon.webSiteURL })
			logger.info('Successfully called open_url')
		}
		catch (e)
		{
			logger.error('Failed to open URL', { url: addon.webSiteURL, error: e })
		}
	}
	else
	{
		logger.warn('No valid webSiteURL found for addon', {
			addonName: addon.addon_name,
			webSiteURL: addon.webSiteURL,
			webSiteURLType: typeof addon.webSiteURL
		})
	}
}
</script>

<template>
  <div>
    <h2 class="font-semibold mb-2">
      Manifest Preview
    </h2>
    <div v-if="manifest">
      <div class="tabs tabs-boxed mb-4">
        <button
          v-for="cat in ['mods', 'resourcepacks', 'shaderpacks', 'datapacks']"
          :key="cat"
          class="tab"
          :class="{ 'tab-active': tab === cat }"
          @click="tab = cat as typeof tab"
        >
          {{ cat.charAt(0).toUpperCase() + cat.slice(1) }}
        </button>
      </div>
      <div>
        <ul>
          <li
            v-for="addon in getCategoryAddons(tab)"
            :key="addon.addon_project_id + '-' + addon.version"
            class="flex items-center gap-3 p-2 rounded mb-2 bg-base-200"
            :class="{
              'bg-green-100 text-green-900': getAddonStatus(addon) === 'added',
              'bg-red-100 text-red-900': getAddonStatus(addon) === 'removed',
              'bg-base-200': getAddonStatus(addon) === '',
            }"
          >
            <span class="font-medium">{{ addon.addon_name }}</span>
            <span class="ml-2 text-xs text-gray-500">{{ addon.version }}</span>
            <button
              class="btn btn-xs btn-ghost ml-auto"
              title="Open CurseForge page"
              @click.stop="openCurseforge(addon)"
            >
              <Icon
                name="i-ri:external-link-line"
                class="text-gray-500 hover:text-gray-700"
              />
            </button>
          </li>
        </ul>
      </div>
    </div>
    <div
      v-else
      class="text-gray-400"
    >
      No manifest loaded.
    </div>
  </div>
</template>
