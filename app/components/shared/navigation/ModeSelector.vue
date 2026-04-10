<template>
  <div class="join self-center">
    <button
      class="btn join-item"
      :class="{ 'btn-primary': mode === 'admin' }"
      @click="setMode('admin')"
    >
      <Icon
        name="mdi:shield-account"
        size="1.2rem"
        class="mr-0 sm:mr-2"
      />
      Admin
    </button>
    <button
      class="btn join-item"
      :class="{ 'btn-primary': mode === 'user' }"
      @click="setMode('user')"
    >
      <Icon
        name="mdi:account"
        size="1.2rem"
        class="mr-0 sm:mr-2"
      />
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
