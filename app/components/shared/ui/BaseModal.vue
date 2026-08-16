<template>
  <Teleport to="body">
    <div
      v-if="modelValue"
      class="modal modal-open z-50"
      role="dialog"
      aria-modal="true"
      :aria-labelledby="labelledBy"
    >
      <div
        ref="modalBoxRef"
        class="modal-box"
        :class="boxClass"
        tabindex="-1"
      >
        <slot />
      </div>
      <div
        class="modal-backdrop"
        aria-hidden="true"
        @click="handleBackdropClick"
      />
    </div>
  </Teleport>
</template>

<script setup lang="ts">
interface Props
{
	modelValue: boolean
	/** id of the element (usually the heading) that labels this dialog */
	labelledBy: string
	closeOnBackdrop?: boolean
	closeOnEscape?: boolean
	/** Extra class(es) for the .modal-box element, e.g. a width override */
	boxClass?: string
}

const props = withDefaults(defineProps<Props>(), {
	closeOnBackdrop: true,
	closeOnEscape: true
})

const emit = defineEmits<{
	'update:modelValue': [value: boolean]
}>()

const modalBoxRef = ref<HTMLElement | null>(null)
let lastFocusedElement: HTMLElement | null = null

const FOCUSABLE_SELECTOR = 'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])'

function getFocusableElements(): HTMLElement[]
{
	if (modalBoxRef.value === null) return []
	return Array.from(modalBoxRef.value.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR))
}

function handleBackdropClick()
{
	if (props.closeOnBackdrop)
	{
		emit('update:modelValue', false)
	}
}

// Keeps Tab/Shift+Tab cycling within the dialog instead of escaping to the page
// behind it, and lets Escape dismiss — neither existed on any modal in the app
// before this (F-P2-7); ConfigFilesSection's own Escape-only handling is the
// one exception, folded into this shared implementation.
function handleKeydown(e: KeyboardEvent)
{
	if (e.key === 'Escape' && props.closeOnEscape)
	{
		emit('update:modelValue', false)
		return
	}

	if (e.key !== 'Tab') return

	const focusable = getFocusableElements()
	if (focusable.length === 0)
	{
		e.preventDefault()
		modalBoxRef.value?.focus()
		return
	}

	const first = focusable[0]
	const last = focusable[focusable.length - 1]
	const active = document.activeElement

	if (e.shiftKey)
	{
		if (active === first || !focusable.includes(active as HTMLElement))
		{
			e.preventDefault()
			last?.focus()
		}
	}
	else if (active === last)
	{
		e.preventDefault()
		first?.focus()
	}
}

watch(() => props.modelValue, async (open) =>
{
	if (!import.meta.client) return

	if (open)
	{
		lastFocusedElement = document.activeElement as HTMLElement | null
		document.addEventListener('keydown', handleKeydown)
		document.body.style.overflow = 'hidden'

		await nextTick()
		const focusable = getFocusableElements()
		if (focusable.length > 0)
		{
			focusable[0]?.focus()
		}
		else
		{
			modalBoxRef.value?.focus()
		}
	}
	else
	{
		document.removeEventListener('keydown', handleKeydown)
		document.body.style.overflow = ''
		lastFocusedElement?.focus()
		lastFocusedElement = null
	}
}, { immediate: true })

onUnmounted(() =>
{
	if (import.meta.client)
	{
		document.removeEventListener('keydown', handleKeydown)
		document.body.style.overflow = ''
	}
})
</script>
