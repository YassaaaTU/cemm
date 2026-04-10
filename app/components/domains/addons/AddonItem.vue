<template>
  <div
    class="addon-item p-3 bg-base-200 hover:bg-base-300 transition-colors"
    :class="{
      'ring-2 ring-primary': selected,
      'bg-green-100 text-green-900': status === 'added',
      'bg-red-100 text-red-900': status === 'removed',
      'opacity-50 bg-red-50': excluded,
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
            <h3
              class="font-medium text-sm truncate"
              :class="{ 'line-through text-red-500': excluded }"
            >
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
        <!-- Exclude button (admin mode only) -->
        <button
          v-if="showExclusion"
          class="btn btn-ghost btn-xs"
          :class="{ 'text-red-500': excluded }"
          :title="excluded ? 'Include in upload' : 'Exclude from upload'"
          @click="$emit('toggleExclusion', addon.addon_name)"
        >
          <svg
            class="w-4 h-4"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              v-if="excluded"
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636"
            />
            <path
              v-else
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636"
            />
          </svg>
        </button>

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
	excluded?: boolean
	showExclusion?: boolean
}

interface Emits
{
	toggleSelection: [addonName: string]
	openLink: [addon: Addon]
	toggleExclusion: [addonName: string]
}

defineProps<Props>()
defineEmits<Emits>()
</script>
