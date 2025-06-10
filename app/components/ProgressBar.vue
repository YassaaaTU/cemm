<template>
  <progress
    class="progress w-full"
    :value="progress"
    max="100"
  />
</template>

<script setup lang="ts">
const props = defineProps<{ progress: number }>()
</script>
