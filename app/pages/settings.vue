<template>
  <div class="p-6">
    <button
      class="btn btn-ghost mb-4"
      @click="goBack"
    >
      ← Back
    </button>
    <h1 class="text-2xl font-bold mb-4">
      Settings
    </h1>
    <git-hub-settings />
  </div>
</template>

<script setup lang="ts">
const router = useRouter()
const goBack = () =>
{
	router.back()
}
</script>
