<template>
  <footer class="footer footer-horizontal bg-base-200 text-base-content items-center p-3 border-l border-base-300 ml-56 w-[calc(100vw-14rem)]">
    <aside class="grid-flow-col items-center gap-2">
      <span class="text-primary font-semibold text-lg">
        No-Logo
      </span>
      <div class="divider divider-horizontal m-0" />
      <p>
        MIT License © {{ year }} - All rights reserved
      </p>
    </aside>
    <nav class="grid-flow-col gap-4 md:place-self-center md:justify-self-end">
      <a
        href="https://github.com/YassaaaTU/cemm"
        target="_blank"
        rel="noopener noreferrer"
        aria-label="GitHub Repository"
        class="hover:text-primary transition-colors"
      >
        <Icon
          name="mdi:github"
          size="28"
        />
      </a>
    </nav>
  </footer>
</template>

<script setup lang="ts">
const year = new Date().getFullYear()
</script>
