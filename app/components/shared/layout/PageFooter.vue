<template>
  <footer
    class="app-footer"
    role="contentinfo"
  >
    <div class="app-footer__left">
      <span class="app-footer__mark">C</span>
      <span class="app-footer__name">CEMM</span>
      <span class="app-footer__separator">·</span>
      <span class="app-footer__copy">MIT License © {{ year }}</span>
    </div>
    <nav class="app-footer__right">
      <a
        href="https://github.com/YassaaaTU/cemm"
        target="_blank"
        rel="noopener noreferrer"
        aria-label="GitHub Repository"
        class="app-footer__link"
      >
        <Icon
          name="mdi:github"
          size="20"
        />
      </a>
    </nav>
  </footer>
</template>

<script setup lang="ts">
const year = new Date().getFullYear()
</script>

<style scoped>
.app-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 36px;
  padding: 0 var(--space-4);
  background: var(--color-surface-raised);
  border-top: 1px solid var(--color-border-subtle);
  font-size: var(--text-body-xs-size);
  flex-shrink: 0;
}

.app-footer__left {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  color: var(--color-text-secondary);
}

.app-footer__mark {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border-radius: var(--radius-sm);
  background: var(--color-accent-primary);
  color: var(--color-text-inverse);
  font-family: var(--font-heading);
  font-weight: 700;
  font-size: 0.625rem;
}

.app-footer__name {
  font-family: var(--font-heading);
  font-weight: 700;
  color: var(--color-text-primary);
  font-size: var(--text-body-sm-size);
}

.app-footer__separator {
  color: var(--color-text-tertiary);
}

.app-footer__copy {
  color: var(--color-text-tertiary);
}

.app-footer__right {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}

.app-footer__link {
  color: var(--color-text-tertiary);
  transition: color var(--duration-fast) var(--ease-standard);
  display: inline-flex;
}

.app-footer__link:hover {
  color: var(--color-accent-primary);
}

.app-footer__link:focus-visible {
  box-shadow: var(--focus-ring-offset);
  outline: none;
  border-radius: var(--radius-sm);
}
</style>
