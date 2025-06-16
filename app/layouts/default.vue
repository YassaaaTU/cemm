<template>
  <div
    class="min-h-screen flex bg-base-100 text-base-content font-sans antialiased"
  >
    <!-- Sidebar -->
    <SideBar
      class="sidebar flex-none bg-base-200"
    />

    <div class="flex-1 flex flex-col">
      <!-- Main content area -->
      <main
        role="main"
        class="flex-1 h-0 min-h-0 flex flex-col ml-56 overflow-y-auto"
      >
        <slot />
      </main>

      <!-- Footer -->
      <PageFooter />
    </div>
  </div>
</template>
