<template>
  <div class="p-6 space-y-6 max-w-3xl mx-auto">
    <NuxtLink
      to="/"
      class="btn btn-ghost mb-2"
    >
      ← Back
    </NuxtLink>
    <h1 class="text-2xl font-bold mb-4">
      User Mode
    </h1>
    <div class="form-control w-full max-w-xs">
      <label class="label">
        <span class="label-text">Update UUID</span>
      </label>
      <input
        v-model="uuid"
        type="text"
        class="input input-bordered w-full"
        placeholder="Paste update UUID here"
      />
    </div>
    <file-selector class="mt-4" />
    <manifest-preview class="mt-4" />
    <addon-list class="mt-4" />
    <div class="mt-6 flex flex-col gap-2">
      <progress-bar
        :progress="progress"
      />
      <status-alert
        :message="statusMessage"
        :type="statusType"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import AddonList from '~/components/AddonList.vue'
import FileSelector from '~/components/FileSelector.vue'
import ManifestPreview from '~/components/ManifestPreview.vue'
import ProgressBar from '~/components/ProgressBar.vue'
import StatusAlert from '~/components/StatusAlert.vue'

const uuid = ref('')
const progress = ref(0)
const statusMessage = ref('')
const statusType = ref<'success' | 'error' | 'info' | 'warning'>('info')
</script>
