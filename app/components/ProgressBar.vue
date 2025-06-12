<template>
  <progress
    class="progress w-full"
    :value="progress"
    max="100"
  />
</template>

<script setup lang="ts">
defineProps<{ progress: number }>()
</script>
