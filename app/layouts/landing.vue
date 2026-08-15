<template>
  <div class="landing-shell">
    <slot />
  </div>
</template>

<style scoped>
.landing-shell {
  min-height: 100dvh;
  background: var(--color-surface-base);
  color: var(--color-text-primary);
  font-family: var(--font-sans);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}
</style>
