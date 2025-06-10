<template>
  <div>
    <h2 class="font-semibold mb-2">
      Manifest Preview
    </h2>
    <div v-if="manifest">
      <div>
        <strong>Mods:</strong>
        <ul class="list-disc ml-6">
          <li
            v-for="mod in manifest.mods"
            :key="mod.addon_project_id"
          >
            {{ mod.addon_name }} ({{ mod.version }})
          </li>
        </ul>
      </div>
      <div class="mt-2">
        <strong>Resourcepacks:</strong>
        <ul class="list-disc ml-6">
          <li
            v-for="pack in manifest.resourcepacks"
            :key="pack.addon_project_id"
          >
            {{ pack.addon_name }} ({{ pack.version }})
          </li>
        </ul>
      </div>
      <div class="mt-2">
        <strong>Shaderpacks:</strong>
        <ul class="list-disc ml-6">
          <li
            v-for="shader in manifest.shaderpacks"
            :key="shader.addon_project_id"
          >
            {{ shader.addon_name }} ({{ shader.version }})
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

<script setup lang="ts">
import { computed } from 'vue'

import { useManifestStore } from '~/stores/manifest'

const manifestStore = useManifestStore()
const manifest = computed(() => manifestStore.manifest)
</script>
