<script setup lang="ts">
interface Props {
	customModpackName: string
	manifestLoaded: boolean
	uploading: boolean
	configFilesCount: number
	excludedCount: number
}

defineProps<Props>()

defineEmits<{
	'update:customModpackName': [value: string]
	'loadInstance': []
	'saveManifest': []
	'uploadToGithub': []
	'clearExclusions': []
}>()
</script>

<template>
  <div class="command-bar">
    <div class="command-bar__left">
      <h2 class="command-bar__title">
        Admin Mode
      </h2>
      <span
        v-if="manifestLoaded"
        class="command-bar__status"
      >
        <span
          class="command-bar__status-dot"
          aria-hidden="true"
        />
        Manifest loaded
      </span>
    </div>

    <div class="command-bar__center">
      <div class="command-bar__input">
        <label
          class="label"
          for="command-bar-modpack-name"
        >
          <span class="label-text">Modpack Name</span>
        </label>
        <input
          id="command-bar-modpack-name"
          :value="customModpackName"
          type="text"
          class="input input-bordered input-sm w-full"
          placeholder="Custom modpack name (optional)"
          @input="$emit('update:customModpackName', ($event.target as HTMLInputElement).value)"
        />
      </div>
    </div>

    <div class="command-bar__right">
      <button
        class="btn btn-primary btn-sm"
        @click="$emit('loadInstance')"
      >
        <Icon
          name="mdi:folder-open"
          size="1.1rem"
        />
        Load Instance
      </button>

      <button
        class="btn btn-outline btn-sm"
        :disabled="!manifestLoaded"
        @click="$emit('saveManifest')"
      >
        <Icon
          name="mdi:content-save"
          size="1.1rem"
        />
        Save Manifest
      </button>

      <button
        class="btn btn-primary btn-sm"
        :disabled="(!manifestLoaded && configFilesCount === 0) || uploading"
        @click="$emit('uploadToGithub')"
      >
        <span
          v-if="uploading"
          class="loading loading-spinner loading-xs"
        />
        <Icon
          v-else
          name="mdi:cloud-upload"
          size="1.1rem"
        />
        <span>{{ uploading ? 'Uploading...' : 'Upload to GitHub' }}</span>
        <span
          v-if="configFilesCount > 0"
          class="command-bar__badge"
        >+{{ configFilesCount }} config</span>
      </button>
    </div>

    <!-- Exclusion quick action (visible when items excluded) -->
    <Transition name="slide-down">
      <div
        v-if="excludedCount > 0"
        class="command-bar__exclusion-bar"
      >
        <div class="command-bar__exclusion-info">
          <Icon
            name="mdi:information-outline"
            size="1rem"
          />
          <span><strong>{{ excludedCount }}</strong> addon(s) excluded from upload</span>
        </div>
        <button
          class="btn btn-ghost btn-sm"
          @click="$emit('clearExclusions')"
        >
          <Icon
            name="mdi:undo"
            size="1.1rem"
          />
          Clear All
        </button>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.command-bar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-4);
  padding: var(--space-4) var(--space-5);
  background: var(--color-surface-raised);
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-lg);
  box-shadow: var(--elev-1);
  margin-bottom: var(--space-6);
  position: relative;
}

.command-bar__left {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.command-bar__title {
  font-family: var(--font-heading);
  font-size: var(--text-heading-md-size);
  font-weight: 700;
  color: var(--color-text-primary);
  margin: 0;
  letter-spacing: var(--text-heading-md-letter-spacing);
}

.command-bar__status {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  font-size: var(--text-body-xs-size);
  color: var(--color-status-success);
}

.command-bar__status-dot {
  width: 6px;
  height: 6px;
  border-radius: var(--radius-full);
  background: var(--color-status-success);
  animation: pulse-ring 1.5s infinite;
}

.command-bar__center {
  flex: 1;
  min-width: 200px;
}

.command-bar__input {
  max-width: 320px;
}

.command-bar__right {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-wrap: wrap;
}

.command-bar__badge {
  display: inline-flex;
  align-items: center;
  padding: 1px 6px;
  border-radius: var(--radius-sm);
  background: var(--color-accent-secondary-muted);
  color: var(--color-accent-secondary);
  font-size: var(--text-body-xs-size);
  font-weight: 700;
  margin-left: var(--space-1);
}

.command-bar__exclusion-bar {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-2) var(--space-3);
  background: var(--color-status-warning-muted);
  border-radius: var(--radius-md);
  margin-top: var(--space-2);
}

.command-bar__exclusion-info {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--text-body-sm-size);
  color: var(--color-status-warning);
}

/* Slide-down transition */
.slide-down-enter-active { transition: all var(--duration-normal) var(--ease-out); }
.slide-down-leave-active { transition: all var(--duration-fast) var(--ease-in); }
.slide-down-enter-from, .slide-down-leave-to { opacity: 0; transform: translateY(-8px); max-height: 0; overflow: hidden; }

/* Responsive */
@media (max-width: 768px) {
  .command-bar {
    flex-direction: column;
    align-items: stretch;
  }
  .command-bar__center {
    min-width: 100%;
  }
  .command-bar__input {
    max-width: 100%;
  }
  .command-bar__right {
    justify-content: flex-end;
  }
}

@media (prefers-reduced-motion: reduce) {
  .command-bar__status-dot {
    animation: none;
  }
}
</style>
