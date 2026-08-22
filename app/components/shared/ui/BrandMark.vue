<template>
  <svg
    viewBox="1.65 0.7 20.7 20.7"
    fill="currentColor"
    role="img"
    :aria-label="title"
  >
    <!-- This comment, and the ones below, must stay INSIDE the <svg>.
         A comment beside the root element makes the template a multi-root
         fragment, and Vue silently stops forwarding `class` when a component
         has more than one root. Dev builds hid it; production did not, and both
         call sites lost their sizing — the title-bar mark collapsed to 0x0 and
         the About mark rendered at 670px.

         An isometric box with its lid lifted clear of the body: one pack, and
         the part that is arriving to join it. CEMM never ships a whole modpack,
         only the difference to one you already have, so the mark is a container
         receiving something rather than a closed cube.

         Faces are filled rather than outlined because this is drawn at 17px in
         the title bar, where a 1.4px stroke closes up into a smudge. The same
         geometry, on a violet tile, is `public/brand/cemm-icon.svg` — the source
         the whole Tauri icon set is generated from, so the window icon and the
         mark inside the window are the one shape. -->
    <title>{{ title }}</title>

    <!-- Lid — the arriving part. -->
    <path d="M12 0.7 21 5.1 12 10.3 3 5.1Z" />

    <!-- Body, left face. -->
    <path d="M3 7 12 12.2 12 21.4 3 17Z" />

    <!-- Body, right face. Held back so the two planes read as separate
         without a shadow, a bevel or a stroke; the theme runs at zero depth. -->
    <path
      d="M21 7 21 17 12 21.4 12 12.2Z"
      fill-opacity="0.72"
    />
  </svg>
</template>

<script setup lang="ts">
withDefaults(defineProps<{ title?: string }>(), {
	title: 'CEMM'
})
</script>
