<template>
  <div>
    <button
      class="btn"
      @click="select"
    >
      Select Directory
    </button>
    <span
      v-if="path"
      class="ml-2"
    >{{ path }}</span>
  </div>
</template>

<script setup lang="ts">
import { useTauri } from '~/composables/useTauri'
import { useAppStore } from '~/stores/app'

const appStore = useAppStore()
const path = computed(() => appStore.modpackPath)
const { selectDirectory } = useTauri()

const select = async () =>
{
	const result = await selectDirectory()
	if (result !== null && result.length > 0)
	{
		appStore.modpackPath = result
	}
}
</script>
