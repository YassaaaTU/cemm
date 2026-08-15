<template>
  <div
    class="addon-card"
    :class="[
      `addon-card--${layout}`,
      {
        'addon-card--selected': selected,
        'addon-card--added': status === 'added',
        'addon-card--removed': status === 'removed',
        'addon-card--updated': status === 'updated',
        'addon-card--excluded': excluded,
      },
    ]"
  >
    <input
      v-if="showSelection"
      type="checkbox"
      :checked="selected"
      class="addon-card__checkbox checkbox checkbox-sm"
      :aria-label="`Select ${addon.addon_name}`"
      @change="$emit('toggleSelection', addon.addon_name)"
    />

    <div class="addon-card__info">
      <div class="addon-card__header">
        <h3
          class="addon-card__name"
          :class="{ 'addon-card__name--excluded': excluded }"
        >
          {{ addon.addon_name }}
        </h3>
        <span
          v-if="status === 'added'"
          class="addon-card__badge addon-card__badge--new"
        >NEW</span>
        <span
          v-else-if="status === 'updated'"
          class="addon-card__badge addon-card__badge--updated"
        >UPDATED</span>
        <span
          v-else-if="status === 'removed'"
          class="addon-card__badge addon-card__badge--removed"
        >REMOVED</span>
        <span
          v-else-if="excluded"
          class="addon-card__badge addon-card__badge--excluded"
        >EXCLUDED</span>
      </div>
      <p class="addon-card__version">
        v{{ addon.version }}
      </p>
    </div>

    <div class="addon-card__actions">
      <div
        v-if="showExclusion"
        class="tooltip tooltip-top"
        :data-tip="excluded ? 'Include in upload' : 'Exclude from upload'"
      >
        <button
          class="addon-card__action"
          :class="{ 'addon-card__action--active': excluded }"
          :aria-label="excluded ? 'Include in upload' : 'Exclude from upload'"
          @click="$emit('toggleExclusion', addon.addon_name)"
        >
          <Icon
            :name="excluded ? 'mdi:close-circle' : 'mdi:minus-circle-outline'"
            size="1.1rem"
          />
        </button>
      </div>

      <div
        v-if="addon.webSiteURL"
        class="tooltip tooltip-top"
        data-tip="Open addon page"
      >
        <button
          class="addon-card__action"
          aria-label="Open addon page"
          @click="$emit('openLink', addon)"
        >
          <Icon
            name="mdi:open-in-new"
            size="0.9rem"
          />
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
	selected?: boolean
	showSelection?: boolean
	status?: '' | 'added' | 'removed' | 'updated'
	excluded?: boolean
	showExclusion?: boolean
	layout?: 'grid' | 'list'
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

<style scoped>
.addon-card {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  background: var(--color-surface-raised);
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-md);
  transition: all var(--duration-fast) var(--ease-standard);
  position: relative;
  overflow: hidden;
}

.addon-card--added { border-color: var(--color-status-success); }
.addon-card--removed { border-color: var(--color-status-error); }
.addon-card--updated { border-color: var(--color-status-warning); }

.addon-card--excluded {
  opacity: 0.5;
}
.addon-card--excluded::after {
  content: '';
  position: absolute;
  inset: 0;
  background: repeating-linear-gradient(
    -45deg,
    transparent,
    transparent 4px,
    var(--color-status-error-muted) 4px,
    var(--color-status-error-muted) 5px
  );
  pointer-events: none;
}

.addon-card--selected {
  border-color: var(--color-accent-primary);
  box-shadow: 0 0 0 1px var(--color-accent-primary);
}

.addon-card:hover:not(.addon-card--excluded) {
  background: var(--color-surface-hover);
}

.addon-card--grid {
  flex-direction: column;
  align-items: flex-start;
  min-height: 64px;
}

.addon-card--list {
  flex-direction: row;
  height: 44px;
  border-radius: var(--radius-sm);
}

.addon-card__checkbox {
  flex-shrink: 0;
}

.addon-card__info {
  flex: 1;
  min-width: 0;
}

.addon-card__header {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.addon-card__name {
  font-size: var(--text-body-sm-size);
  font-weight: 600;
  color: var(--color-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.addon-card__name--excluded {
  text-decoration: line-through;
  color: var(--color-status-error);
}

.addon-card__version {
  font-family: var(--font-mono);
  font-size: var(--text-body-xs-size);
  color: var(--color-text-tertiary);
  margin-top: 2px;
}

.addon-card__badge {
  display: inline-flex;
  align-items: center;
  padding: 1px 6px;
  border-radius: var(--radius-sm);
  font-size: 0.625rem;
  font-weight: 700;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  flex-shrink: 0;
}
.addon-card__badge--new {
  background: var(--color-status-success-muted);
  color: var(--color-status-success);
}
.addon-card__badge--updated {
  background: var(--color-status-warning-muted);
  color: var(--color-status-warning);
}
.addon-card__badge--removed {
  background: var(--color-status-error-muted);
  color: var(--color-status-error);
}
.addon-card__badge--excluded {
  background: var(--color-status-error-muted);
  color: var(--color-status-error);
}

.addon-card__actions {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  flex-shrink: 0;
}

.addon-card__action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  color: var(--color-text-tertiary);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--duration-instant);
}
.addon-card__action:hover {
  background: var(--color-surface-hover);
  color: var(--color-text-primary);
}
.addon-card__action--active {
  color: var(--color-status-error);
}
.addon-card__action:focus-visible {
  box-shadow: var(--focus-ring-offset);
  outline: none;
}
</style>
