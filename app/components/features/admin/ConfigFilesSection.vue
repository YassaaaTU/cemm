<template>
  <div class="card bg-base-200 shadow-md border border-base-300 config-files">
    <div class="card-body border-b border-base-300 pb-3">
      <div class="config-files__header">
        <h3 class="config-files__title">
          Config Files (Optional)
        </h3>
        <div
          class="tooltip tooltip-top"
          data-tip="Configuration files will be applied to the user's modpack directory during installation"
        >
          <Icon
            name="mdi:help-circle-outline"
            size="1rem"
            class="config-files__help"
          />
        </div>
      </div>
    </div>

    <div class="card-body">
      <p class="config-files__desc">
        Select configuration files to include with your modpack update.
      </p>

      <div class="config-files__actions">
        <button
          class="btn btn-outline btn-sm"
          @click="handleSelectFiles"
        >
          <Icon
            name="mdi:file-plus"
            size="1.1rem"
          />
          Add Config Files
        </button>
        <button
          class="btn btn-outline btn-sm"
          @click="showDirectorySelector = true"
        >
          <Icon
            name="mdi:folder-plus"
            size="1.1rem"
          />
          Add From Directory
        </button>
        <button
          v-if="modelValue.length > 0"
          class="btn btn-error btn-sm"
          @click="clearFiles"
        >
          <Icon
            name="mdi:trash-can"
            size="1.1rem"
          />
          Clear All
        </button>
      </div>

      <div
        v-if="modelValue.length > 0"
        class="config-files__list"
      >
        <div class="config-files__list-header">
          Selected Files ({{ modelValue.length }}):
        </div>
        <div
          v-for="(configFile, index) in modelValue"
          :key="index"
          class="config-files__item"
        >
          <div class="config-files__item-info">
            <span
              class="config-files__badge"
              :class="configFile.is_binary ? 'config-files__badge--binary' : 'config-files__badge--config'"
            >
              {{ configFile.is_binary ? 'BIN' : 'CFG' }}
            </span>
            <span class="config-files__path">{{ configFile.relative_path }}</span>
            <span class="config-files__size">
              <template v-if="configFile.is_binary">
                {{ configFile.filename.split('.').pop()?.toUpperCase() || 'BINARY' }}
              </template>
              <template v-else>
                {{ Math.round(configFile.content.length / 1024 * 100) / 100 }} KB
              </template>
            </span>
          </div>
          <button
            class="config-files__remove"
            aria-label="Remove file"
            @click="removeFile(configFile)"
          >
            <Icon
              name="mdi:close"
              size="1rem"
            />
          </button>
        </div>
      </div>

      <div
        v-else
        class="config-files__empty"
      >
        <Icon
          name="mdi:file-document-outline"
          size="2.5rem"
          class="config-files__empty-icon"
        />
        <p class="config-files__empty-text">
          No config files selected
        </p>
        <p class="config-files__empty-hint">
          Config files will be applied to the user's modpack directory
        </p>
      </div>
    </div>
  </div>

  <BaseModal
    v-model="showDirectorySelector"
    labelled-by="select-config-directory-title"
    box-class="max-w-lg"
  >
    <h3
      id="select-config-directory-title"
      class="text-lg font-bold"
    >
      Select Config Directory
    </h3>
    <p class="py-1 text-sm opacity-70">
      Choose a directory to scan for config files
    </p>
    <PathSelector
      type="directory"
      title="Select Config Directory"
      @selected="handleDirectorySelected"
      @error="handleDirectoryError"
    />
    <div class="modal-action">
      <button
        class="btn btn-ghost"
        @click="showDirectorySelector = false"
      >
        Cancel
      </button>
    </div>
  </BaseModal>
</template>

<script setup lang="ts">
import type { ConfigFileWithContent } from '~/types'

const props = defineProps<{
	modelValue: ConfigFileWithContent[]
}>()

const emit = defineEmits<{
	'update:modelValue': [value: ConfigFileWithContent[]]
	'status': [message: string, type: 'success' | 'error' | 'info' | 'warning']
}>()

const showDirectorySelector = ref(false)

// Escape, scroll lock, and focus trapping now live in BaseModal.

async function handleSelectFiles()
{
	const { selectConfigFiles } = useAdminApi()
	const newFiles = await selectConfigFiles((message: string, type: 'success' | 'error' | 'info' | 'warning') =>
	{
		emit('status', message, type)
	})
	if (newFiles.length > 0)
	{
		emit('update:modelValue', [...props.modelValue, ...newFiles])
	}
}

async function handleDirectorySelected(dirPath: string | string[])
{
	showDirectorySelector.value = false
	const pathToUse = Array.isArray(dirPath) ? dirPath[0] : dirPath

	if (pathToUse === undefined || pathToUse.trim().length === 0)
	{
		emit('status', 'No directory selected.', 'warning')
		return
	}

	const { scanDirectoryForConfigFiles } = useAdminApi()
	const newFiles = await scanDirectoryForConfigFiles(pathToUse, (message: string, type: 'success' | 'error' | 'info' | 'warning') =>
	{
		emit('status', message, type)
	})
	if (newFiles.length > 0)
	{
		emit('update:modelValue', [...props.modelValue, ...newFiles])
	}
}

function handleDirectoryError(error: string)
{
	showDirectorySelector.value = false
	emit('status', `Directory selection error: ${error}`, 'error')
}

function removeFile(configFile: ConfigFileWithContent)
{
	const index = props.modelValue.findIndex((cf) => cf.relative_path === configFile.relative_path)
	if (index !== -1)
	{
		const newValue = [...props.modelValue]
		newValue.splice(index, 1)
		emit('update:modelValue', newValue)
		emit('status', `Removed config file: ${configFile.relative_path}`, 'info')
	}
}

function clearFiles()
{
	emit('update:modelValue', [])
	emit('status', 'Cleared all config files.', 'info')
}
</script>

<style scoped>
.config-files__header {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.config-files__title {
  font-size: var(--text-heading-sm-size);
  font-weight: 600;
  color: var(--color-text-primary);
  margin: 0;
}

.config-files__help {
  color: var(--color-text-tertiary);
  cursor: help;
}

.config-files__desc {
  font-size: var(--text-body-sm-size);
  color: var(--color-text-secondary);
  margin: 0 0 var(--space-4);
}

.config-files__actions {
  display: flex;
  gap: var(--space-2);
  margin-bottom: var(--space-4);
  flex-wrap: wrap;
}

.config-files__list-header {
  font-size: var(--text-body-sm-size);
  font-weight: 600;
  color: var(--color-text-secondary);
  margin-bottom: var(--space-2);
}

.config-files__list {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.config-files__item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-2) var(--space-3);
  background: var(--color-surface-base);
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border-subtle);
}

.config-files__item-info {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
  flex: 1;
}

.config-files__badge {
  display: inline-flex;
  padding: 1px 6px;
  border-radius: var(--radius-sm);
  font-size: var(--text-body-xs-size);
  font-weight: 700;
  letter-spacing: 0.05em;
  flex-shrink: 0;
}
.config-files__badge--binary {
  background: var(--color-accent-secondary-muted);
  color: var(--color-accent-secondary);
}
.config-files__badge--config {
  background: var(--color-accent-primary-muted);
  color: var(--color-accent-primary);
}

.config-files__path {
  font-family: var(--font-mono);
  font-size: var(--text-body-sm-size);
  color: var(--color-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.config-files__size {
  font-size: var(--text-body-xs-size);
  color: var(--color-text-tertiary);
  flex-shrink: 0;
}

.config-files__remove {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  color: var(--color-status-error);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: background var(--duration-instant);
  flex-shrink: 0;
}
.config-files__remove:hover {
  background: var(--color-status-error-muted);
}
.config-files__remove:focus-visible {
  box-shadow: var(--focus-ring-offset);
  outline: none;
}

.config-files__empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: var(--space-6);
  text-align: center;
}
.config-files__empty-icon {
  color: var(--color-text-tertiary);
  margin-bottom: var(--space-2);
}
.config-files__empty-text {
  font-size: var(--text-body-sm-size);
  color: var(--color-text-secondary);
  margin: 0;
}
.config-files__empty-hint {
  font-size: var(--text-body-xs-size);
  color: var(--color-text-tertiary);
  margin: var(--space-1) 0 0;
}
</style>
