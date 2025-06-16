<template>
  <div>
    <div
      class="mt-3 join w-full"
    >
      <button
        class="btn btn-primary join-item"
        @click="select"
      >
        <Icon
          name="mdi:folder-open"
          class="mr-2"
        />
        Select Directory
      </button>
      <div
        class="input input-bordered flex items-center gap-2 w-full overflow-hidden join-item"
        :title="path"
      >
        <span class="truncate flex-1 select-text">{{ (path != '' || null) ? path : 'No directory selected' }}</span>
        <button
          class="btn btn-xs btn-circle btn-ghost"
          title="Clear"
          @click="clearDirectory"
        >
          <Icon
            name="solar:close-circle-linear"
            size="1.8em"
          />
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useAppStore } from '~/stores/app'

const appStore = useAppStore()
const { selectDirectory } = useTauri()
const logger = usePinoLogger()

const path = computed(() => appStore.modpackPath)

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

function clearDirectory()
{
	appStore.modpackPath = ''
}
</script>
