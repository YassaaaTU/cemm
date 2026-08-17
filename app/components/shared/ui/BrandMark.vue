<template>
  <!-- An isometric cube split along its top face: one pack, assembled from
       parts that arrive separately. Drawn rather than pulled from an icon set
       because it is the product's identity. -->
  <svg
    viewBox="0 0 24 24"
    fill="none"
    role="img"
    :aria-label="title"
  >
    <title>{{ title }}</title>
    <path
      d="M12 2.6 21 7v10l-9 4.4L3 17V7l9-4.4Z"
      stroke="currentColor"
      stroke-width="1.7"
      stroke-linejoin="round"
    />
    <path
      d="M12 12.2 21 7.6M12 12.2v9M12 12.2 3 7.6"
      stroke="currentColor"
      stroke-width="1.4"
      stroke-linejoin="round"
    />
  </svg>
</template>

<script setup lang="ts">
withDefaults(defineProps<{ title?: string }>(), {
	title: 'CEMM'
})
</script>
