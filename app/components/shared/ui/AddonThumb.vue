<template>
  <!-- CurseForge and Modrinth both lead each row with the project icon. CEMM's
       Addon type carries an optional thumbnailUrl, but those load from the
       CurseForge CDN and this app has to work offline, so a coloured initial
       tile is the fallback rather than a broken image. The tile colour is
       derived from the addon name, which keeps a given mod visually stable
       across sessions without storing anything. -->
  <div
    v-if="showImage"
    class="avatar shrink-0"
  >
    <div class="w-8 rounded-md">
      <img
        :src="src"
        :alt="''"
        width="64"
        height="64"
        class="size-full object-cover"
        loading="lazy"
        @error="failed = true"
      />
    </div>
  </div>

  <div
    v-else
    class="avatar avatar-placeholder shrink-0"
    aria-hidden="true"
  >
    <div
      class="w-8 rounded-md text-base-content"
      :class="tint"
    >
      <span class="text-[0.8125rem] font-bold">{{ initial }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
const props = withDefaults(
	defineProps<{ name: string, src?: string }>(),
	{ src: '' }
)

const failed = ref(false)

const showImage = computed(() => props.src.length > 0 && !failed.value)

const initial = computed(() =>
{
	const first = props.name.trim().charAt(0)
	return first.length > 0 ? first.toUpperCase() : '?'
})

/**
 * A small fixed set of theme-safe tints, picked by a stable hash of the name so
 * the same addon always gets the same tile.
 */
const tint = computed(() =>
{
	const tints = [
		'bg-primary/25',
		'bg-info/25',
		'bg-success/25',
		'bg-warning/25',
		'bg-secondary/25',
		'bg-base-300'
	]
	let hash = 0
	for (let index = 0; index < props.name.length; index++)
	{
		hash = (hash * 31 + props.name.charCodeAt(index)) | 0
	}
	return tints[Math.abs(hash) % tints.length]
})
</script>
