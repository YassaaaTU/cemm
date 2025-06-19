<template>
  <div
    class="h-screen flex bg-base-100 text-base-content font-sans antialiased overflow-hidden"
  >
    <!-- Sidebar -->
    <SideBar
      class="sidebar flex-none bg-base-200"
    />

    <!-- Right side content area -->
    <div class="flex-1 flex flex-col min-w-0 min-h-0">
      <!-- Main content area -->
      <main
        role="main"
        class="flex-1 min-h-0 ml-56 overflow-auto"
      >
        <slot />
      </main>
      <!-- Footer -->
      <PageFooter class="flex-none" />
    </div>    <!-- Global Update Dialog -->
    <UpdateDialog
      :is-visible="isUpdateDialogVisible"
      :title="dialogTitle"
      :message="dialogMessage"
      :current-version="currentVersion"
      :new-version="newVersion"
      :is-downloading="isDownloading"
      :download-progress="downloadProgress"
      @confirm="handleUpdateConfirm"
      @cancel="handleUpdateCancel"
    />
  </div>
</template>

<script setup lang="ts">
const {
	isUpdateDialogVisible,
	isDownloading,
	downloadProgress,
	dialogTitle,
	dialogMessage,
	currentVersion,
	newVersion,
	handleUpdateConfirm,
	handleUpdateCancel
} = useUpdater()
</script>
