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
const logger = usePinoLogger()

const select = async () =>
{
	const result = await selectDirectory()
	logger.info('🔍 FileSelector: Raw result from selectDirectory:', { result })
	logger.info('🔍 FileSelector: Result length:', { length: result?.length })
	logger.info('🔍 FileSelector: Result includes temp?', { includesTemp: result?.includes('temp') })

	if (result !== null && result.length > 0)
	{
		logger.info('🔍 FileSelector: Setting modpackPath to:', { result })
		appStore.modpackPath = result
		logger.info('🔍 FileSelector: App store modpackPath after setting:', { modpackPath: appStore.modpackPath })
	}
}
</script>
