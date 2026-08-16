<template>
  <div class="app-shell">
    <!-- Sidebar -->
    <SideBar class="app-shell__sidebar" />

    <!-- Right side content area -->
    <div class="app-shell__main">
      <!-- Breadcrumb area -->
      <div
        v-if="$slots.breadcrumb"
        class="app-shell__breadcrumb"
      >
        <slot name="breadcrumb" />
      </div>

      <!-- Main content area. The single <main> landmark for every page using this
           layout (dashboard.vue, settings.vue) — page-level content must not add
           its own role="main"/<main>, or screen-reader landmark navigation sees
           multiple nested "main" regions (F-P2-8). id/tabindex give a skip-link
           target, shared across pages rather than duplicated per-page. -->
      <main
        id="main-content"
        tabindex="-1"
        class="app-shell__content scrollbar-thin"
      >
        <slot />
      </main>

      <!-- Footer -->
      <PageFooter class="app-shell__footer" />
    </div>

    <!-- Global Update Dialog -->
    <UpdateDialog />
  </div>
</template>

<style scoped>
.app-shell {
  height: 100dvh;
  display: flex;
  background: var(--color-surface-base);
  color: var(--color-text-primary);
  font-family: var(--font-sans);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  overflow: hidden;
}

.app-shell__sidebar {
  flex: none;
}

.app-shell__main {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  margin-left: 200px;
}

.app-shell__breadcrumb {
  flex: none;
  padding: var(--space-3) var(--space-6);
  border-bottom: 1px solid var(--color-border-divider);
  background: var(--color-surface-raised);
}

.app-shell__content {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding: var(--space-6);
}

.app-shell__footer {
  flex: none;
}
</style>
