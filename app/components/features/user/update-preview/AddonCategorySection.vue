<template>
  <div class="collapse collapse-arrow bg-base-200">
    <input
      type="checkbox"
      checked
    />
    <div class="collapse-title font-medium flex items-center gap-2">
      <span class="text-lg">{{ icon }}</span>
      <span>{{ title }}</span>
      <span class="badge badge-ghost badge-sm">{{ sortedItems.length }}</span>
    </div>
    <div class="collapse-content">
      <div class="space-y-2 pt-2">
        <div
          v-for="itemName in sortedItems"
          :key="itemName"
          class="flex items-center justify-between p-3 rounded"
          :class="itemClass"
        >
          <div class="flex items-center space-x-2">
            <span
              class="w-4 h-4 rounded-full flex items-center justify-center"
              :class="iconBgClass"
            >
              <span class="text-xs text-white">{{ itemIcon }}</span>
            </span>
            <span>{{ itemName }}</span>
          </div>
          <span
            class="badge"
            :class="badgeClass"
          >
            {{ badgeText }}
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
interface Props
{
	title: string
	icon: string
	items: string[]
	type: 'new' | 'updated' | 'removed'
}

const props = defineProps<Props>()

// Alphabetically sorted items
const sortedItems = computed(() =>
{
	return [...props.items].sort((a, b) => a.localeCompare(b))
})

// Computed styles based on type
const itemClass = computed(() =>
{
	switch (props.type)
	{
		case 'new':
			return 'bg-success/10 border border-success/20'
		case 'updated':
			return 'bg-warning/10 border border-warning/20'
		case 'removed':
			return 'bg-error/10 border border-error/20'
		default:
			return ''
	}
})

const iconBgClass = computed(() =>
{
	switch (props.type)
	{
		case 'new':
			return 'bg-success'
		case 'updated':
			return 'bg-warning'
		case 'removed':
			return 'bg-error'
		default:
			return ''
	}
})

const itemIcon = computed(() =>
{
	switch (props.type)
	{
		case 'new':
			return '+'
		case 'updated':
			return '↑'
		case 'removed':
			return '−'
		default:
			return ''
	}
})

const badgeClass = computed(() =>
{
	switch (props.type)
	{
		case 'new':
			return 'badge-success'
		case 'updated':
			return 'badge-warning'
		case 'removed':
			return 'badge-error'
		default:
			return ''
	}
})

const badgeText = computed(() =>
{
	switch (props.type)
	{
		case 'new':
			return 'NEW'
		case 'updated':
			return 'UPDATED'
		case 'removed':
			return 'REMOVE'
		default:
			return ''
	}
})
</script>
