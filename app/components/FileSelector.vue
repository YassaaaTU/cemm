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

const path = ref('')
const { selectDirectory } = useTauri()

const select = async () =>
{
	const result = await selectDirectory()
	if (result !== null && result.length > 0)
	{
		path.value = result
	}
}
</script>
