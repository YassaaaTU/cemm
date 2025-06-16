<template>
  <div
    class="min-h-screen flex bg-base-100 text-base-content font-sans antialiased"
  >
    <!-- Sidebar -->
    <SideBar
      class="sidebar flex-none bg-base-200"
    />

    <!-- Main content area -->
    <main
      role="main"
      class="flex-1 min-h-screen flex flex-col ml-56"
    >
      <slot />
    </main>
  </div>
</template>

<script setup lang="ts">

</script>

<style scoped>

</style>
