<template>
  <div>
    <h2 class="text-2xl font-bold mb-4">
      Admin Mode
    </h2>
    <file-selector />
    <addon-list class="mt-4" />
    <manifest-preview class="mt-4" />
    <git-hub-settings class="mt-4" />
    <div class="mt-6 flex flex-col gap-2">
      <progress-bar :progress="progress" />
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
import GitHubSettings from '~/components/GitHubSettings.vue'
import ManifestPreview from '~/components/ManifestPreview.vue'
import ProgressBar from '~/components/ProgressBar.vue'
import StatusAlert from '~/components/StatusAlert.vue'

const progress = ref(0)
const statusMessage = ref('')
const statusType = ref<'success' | 'error' | 'info' | 'warning'>('info')
</script>
