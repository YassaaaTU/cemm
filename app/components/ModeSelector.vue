<template>
  <div class="join">
    <button
      class="btn join-item"
      :class="{ 'btn-primary': mode === 'admin' }"
      @click="setMode('admin')"
    >
      Admin
    </button>
    <button
      class="btn join-item"
      :class="{ 'btn-primary': mode === 'user' }"
      @click="setMode('user')"
    >
      User
    </button>
  </div>
</template>

<script setup lang="ts">
const appStore = useAppStore()
const mode = computed(() => appStore.mode)
const setMode = (val: 'admin' | 'user') =>
{
	appStore.mode = val
}
</script>
