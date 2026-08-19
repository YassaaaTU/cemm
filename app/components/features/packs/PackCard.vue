<template>
  <!--
    A card, because this is the one screen in CEMM whose job is recognition
    rather than reading: you are picking a pack out of forty, and an icon plus a
    name is how both reference apps let you do that.

    Explicit `flex flex-col` and `mt-auto`: a native <button> centres its own
    content whatever its display says, which is what made the setup cards look
    crooked when their descriptions ran to different lengths. Cards here have
    the same problem — a pack with no group and no CEMM history is two lines
    shorter than one with both.
  -->
  <button
    type="button"
    class="group flex h-full cursor-pointer flex-col gap-2.5 rounded-box border bg-base-200 p-3.5 text-left transition-colors duration-150 ease-(--ease-standard) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
    :class="pack.missing
      ? 'border-base-300 opacity-70 hover:border-warning/60'
      : 'border-base-300 hover:border-primary/60 hover:bg-base-300/50'"
    :disabled="busy"
    @click="emit('choose', pack)"
  >
    <div class="flex min-w-0 items-start gap-2.5">
      <span class="grid size-11 shrink-0 place-items-center overflow-hidden rounded-box border border-base-300 bg-base-100">
        <img
          v-if="pack.icon !== null"
          :src="pack.icon"
          alt=""
          class="size-full object-cover"
        />
        <span
          v-else
          class="text-base font-bold"
          :class="initialTone"
          aria-hidden="true"
        >{{ initial }}</span>
      </span>

      <span class="min-w-0 flex-1">
        <span
          class="block truncate text-[0.9375rem] leading-snug font-semibold"
          :title="pack.name"
        >{{ pack.name }}</span>
        <!-- A missing pack cannot be opened, so its click does the only useful
             thing left and the card has to say so before it is pressed. -->
        <span class="mt-0.5 block truncate font-mono text-[0.6875rem] text-base-content/60">
          <template v-if="pack.missing">click to remove from this list</template>
          <template v-else>{{ pack.addonCount }} addons</template>
        </span>
      </span>
    </div>

    <div class="flex flex-wrap items-center gap-x-2 gap-y-1 text-[0.6875rem] text-base-content/60">
      <span
        v-if="pack.gameVersion !== null"
        class="font-mono tabular-nums"
      >{{ pack.gameVersion }}</span>
      <span
        v-if="pack.gameVersion !== null && pack.loader !== null"
        class="text-base-content/30"
        aria-hidden="true"
      >·</span>
      <span v-if="pack.loader !== null">{{ pack.loader }}</span>
      <span
        v-if="pack.gameVersion === null && pack.loader === null"
        class="text-base-content/40"
      >version unknown</span>
    </div>

    <!--
      The folder, always. A library really does hold `FTB Evolution` beside
      `FTB Evolution (1)`, and a folder named `All the Mods 10 - ATM10 (2)`
      whose pack is called `Aeronautics` — a card showing only the name would
      be pointing at something the user cannot identify.
    -->
    <div
      class="truncate font-mono text-[0.6875rem] text-base-content/45"
      :title="pack.instancePath"
    >
      {{ pack.folderName }}
    </div>

    <div class="mt-auto flex flex-wrap items-center gap-1.5 pt-0.5">
      <span
        v-if="groupName !== null"
        class="inline-flex max-w-full items-center truncate rounded-full border border-base-300 bg-base-100 px-2 py-px text-[0.6875rem] text-base-content/70"
      >{{ groupName }}</span>
      <StatusChip
        v-if="historyLabel !== null"
        tone="unchanged"
        :label="historyLabel"
      />
      <StatusChip
        v-if="pack.missing"
        tone="removed"
        label="Missing"
      />
    </div>
  </button>
</template>

<script setup lang="ts">
import type { PackRow } from '~/stores/packs'

const props = defineProps<{
	pack: PackRow
	groupName: string | null
	/** Which timestamp to surface — the two counters care about different ones. */
	mode: 'admin' | 'user'
	busy?: boolean
}>()

const emit = defineEmits<{ choose: [pack: PackRow] }>()

const initial = computed(() => props.pack.name.trim().charAt(0).toUpperCase() || '?')

/**
 * The same offline-safe coloured initial AddonThumb uses for addons without an
 * icon — derived from the name so a pack keeps its colour between launches.
 * Only 3 of the author's 36 instances have an icon CurseForge stored locally,
 * so this is the common case, not the fallback.
 */
const initialTone = computed(() =>
{
	const tones = [
		'text-primary',
		'text-success',
		'text-info',
		'text-warning',
		'text-error',
		'text-secondary',
		'text-accent'
	]
	let hash = 0
	for (const char of props.pack.name) hash = (hash * 31 + char.charCodeAt(0)) % 997
	return tones[hash % tones.length]
})

const relative = (iso: string | undefined): string | null =>
{
	if (iso === undefined) return null
	const then = Date.parse(iso)
	if (!Number.isFinite(then)) return null
	const days = Math.floor((Date.now() - then) / 86_400_000)
	if (days <= 0) return 'today'
	if (days === 1) return 'yesterday'
	if (days < 30) return `${days}d ago`
	if (days < 365) return `${Math.floor(days / 30)}mo ago`
	return `${Math.floor(days / 365)}y ago`
}

/**
 * What CEMM has done with this pack, in the tense that matters to whoever is
 * looking: an admin wants to know when they last shipped from it, a player when
 * they last installed into it.
 */
const historyLabel = computed(() =>
{
	const entry = props.pack.history
	if (entry === null) return null

	// Every one of these is tried, not just the mode's own — a pack sorts to the
	// front of the library because CEMM has touched it, and a card that leads the
	// grid with nothing saying why is the list keeping a secret. The mode only
	// decides which reason is the most relevant one to show first.
	const candidates: Array<[string, string | undefined]> = props.mode === 'admin'
		? [
			['Published', entry.lastPublishedAt],
			['Installed', entry.lastInstalledAt],
			['Opened', entry.lastOpenedAt]
		]
		: [
			['Updated', entry.lastInstalledAt],
			['Published', entry.lastPublishedAt],
			['Opened', entry.lastOpenedAt]
		]

	for (const [verb, stamp] of candidates)
	{
		const when = relative(stamp)
		if (when !== null) return `${verb} ${when}`
	}
	return null
})
</script>
