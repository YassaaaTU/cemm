<template>
  <div
    class="addon-item p-3 bg-base-200 hover:bg-base-300 transition-colors"
    :class="{
      'ring-2 ring-primary': selected,
      'bg-green-100 text-green-900': status === 'added',
      'bg-red-100 text-red-900': status === 'removed',
    }"
  >
    <div class="flex items-center justify-between">
      <!-- Addon info -->
      <div class="flex-1 min-w-0">
        <div class="flex items-center gap-2">
          <!-- Selection checkbox -->
          <input
            v-if="showSelection"
            type="checkbox"
            :checked="selected"
            class="checkbox checkbox-sm"
            @change="$emit('toggleSelection', addon.addon_name)"
          />

          <!-- Addon name and version -->
          <div class="min-w-0 flex-1">
            <h3 class="font-medium text-sm truncate">
              {{ addon.addon_name }}
            </h3>
            <p class="text-xs text-base-content opacity-60">
              v{{ addon.version }}
            </p>
          </div>
        </div>
      </div>

      <!-- Actions -->
      <div class="flex items-center gap-2 ml-2">
        <!-- External link button -->
        <button
          v-if="addon.webSiteURL"
          class="btn btn-ghost btn-xs"
          title="Open CurseForge page"
          @click="$emit('openLink', addon)"
        >
          <svg
            class="w-3 h-3"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"
            />
          </svg>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { Addon } from '~/types'

interface Props
{
	addon: Addon
	index: number
	selected?: boolean
	showSelection?: boolean
	status?: '' | 'added' | 'removed'
}

interface Emits
{
	toggleSelection: [addonName: string]
	openLink: [addon: Addon]
}

defineProps<Props>()
defineEmits<Emits>()
</script>
