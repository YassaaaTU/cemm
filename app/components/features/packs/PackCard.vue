<template>
  <!--
    A card, because this is the one screen in CEMM whose job is recognition
    rather than reading: you are picking a pack out of forty, and an icon plus a
    name is how both reference apps let you do that.

    The card is NOT a control. It used to be one big button whose meaning came
    from whichever counter you happened to be in — the same click installed or
    published depending on invisible state set on another screen. Both actions
    are now named on the card itself, and choosing one is what decides the mode
    rather than the other way round.

    Explicit `flex flex-col` and `mt-auto`: a native button centres its own
    content whatever its display says, which is what made the setup cards look
    crooked. Cards here have the same problem — a pack with no group and no CEMM
    history is two lines shorter than one with both — so the actions are pinned
    to the floor and every card in a row ends level.
  -->
  <div
    class="flex h-full flex-col gap-2.5 rounded-box border bg-base-200 p-3.5 transition-colors duration-150 ease-(--ease-standard)"
    :class="pack.presence === 'missing' ? 'border-base-300 opacity-80' : 'border-base-300'"
  >
    <div class="flex min-w-0 items-start gap-2.5">
      <span class="grid size-11 shrink-0 place-items-center overflow-hidden rounded-box border border-base-300 bg-base-100">
        <img
          v-if="pack.icon !== null && !iconFailed"
          :src="pack.icon"
          alt=""
          class="size-full object-cover"
          @error="iconFailed = true"
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
        <span class="mt-0.5 block truncate font-mono text-[0.6875rem] text-base-content/60">
          <template v-if="presenceNote !== null">{{ presenceNote }}</template>
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

    <div
      v-if="groupName !== null || historyLabel !== null || pack.presence === 'missing'"
      class="flex flex-wrap items-center gap-1.5"
    >
      <span
        v-if="groupName !== null"
        class="inline-flex max-w-full items-center truncate rounded-full border border-base-300 bg-base-100 px-2 py-px text-[0.6875rem] text-base-content/70"
      >{{ groupName }}</span>
      <StatusChip
        v-if="historyLabel !== null"
        tone="unchanged"
        :label="historyLabel"
      />
      <!-- Only for a pack that was actually looked for and not found. A pack
           living outside the scanned folder is not a problem to flag; it is
           just a pack the user chose from somewhere else. -->
      <StatusChip
        v-if="pack.presence === 'missing'"
        tone="removed"
        label="Missing"
      />
    </div>

    <!--
      Both actions, named, on every card. Two is few enough to show outright: a
      menu is for when actions are many or secondary, and these are neither —
      they are the only two things a pack is for. The icons are the rail's own,
      so a button and the destination it leads to look like the same thing.
    -->
    <div class="mt-auto flex gap-1.5 pt-0.5">
      <!--
        Removal is offered only where the folder was checked for and is not
        there. A pack that is merely outside the scanned library still works, so
        it keeps both its actions; and an unchecked pack — which is every
        remembered pack after a scan that failed — is not offered a control that
        would only ever be refused.
      -->
      <template v-if="pack.presence === 'missing'">
        <button
          type="button"
          class="btn w-full cursor-pointer gap-1.5 border-base-300 btn-sm"
          :disabled="isBusy"
          @click="emit('forget', pack)"
        >
          <Icon
            name="mdi:close"
            size="0.9375rem"
            aria-hidden="true"
          />
          Remove from list
          <span class="sr-only">— {{ pack.name }}</span>
        </button>
      </template>

      <template v-else>
        <button
          type="button"
          class="btn flex-1 cursor-pointer gap-1.5 border-base-300 btn-sm"
          :disabled="isBusy"
          @click="emit('install', pack)"
        >
          <Icon
            name="mdi:tray-arrow-down"
            size="0.9375rem"
            aria-hidden="true"
          />
          Install
          <span class="sr-only">an update into {{ pack.name }}</span>
        </button>

        <button
          type="button"
          class="btn flex-1 cursor-pointer gap-1.5 border-base-300 btn-sm"
          :disabled="isBusy"
          @click="emit('publish', pack)"
        >
          <span
            v-if="busy === 'publish'"
            class="loading loading-xs loading-spinner"
            aria-hidden="true"
          />
          <Icon
            v-else
            name="mdi:tray-arrow-up"
            size="0.9375rem"
            aria-hidden="true"
          />
          Publish
          <span class="sr-only">an update from {{ pack.name }}</span>
        </button>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { PackRow } from '~/stores/packs'

const props = withDefaults(
	defineProps<{
		pack: PackRow
		groupName: string | null
		/**
		 * Whether this card's action is in flight. Only publish can be: install
		 * opens a dialog, which is instant and has nothing to wait behind.
		 */
		busy?: 'publish' | false
	}>(),
	{ busy: false }
)

const emit = defineEmits<{
	install: [pack: PackRow]
	publish: [pack: PackRow]
	forget: [pack: PackRow]
}>()

/**
 * `busy` names *which* action is running, because the spinner has to sit on that
 * button and no other. Every `disabled` binding only wants to know whether one
 * is — and handing a string union straight to `disabled`, which takes a boolean,
 * is a type error under a template check strict enough to look.
 */
const isBusy = computed(() => props.busy !== false)

const initial = computed(() => props.pack.name.trim().charAt(0).toUpperCase() || '?')

/**
 * A `data:` URI the browser could not decode, which is the fourth artwork
 * state — the comment on `initialTone` names three and says they look alike.
 * The Rust side now refuses a non-image body before it reaches the cache, so
 * this should not happen; it is here because the card must not be the place
 * that finds out, the way AddonThumb already does not.
 */
const iconFailed = ref(false)
// A card is reused as the grid re-sorts, so a failure must not follow the
// component onto a different pack's picture.
watch(() => props.pack.icon, () =>
{
	iconFailed.value = false
})

/**
 * What to say instead of an addon count for a pack the scan did not return.
 * Each case says only what CEMM actually knows: `unchecked` has not been looked
 * at yet, so it reports the one fact in evidence — the scan did not list it —
 * rather than guessing at why.
 */
const presenceNote = computed<string | null>(() =>
{
	if (props.pack.presence === 'missing') return 'not found on disk'
	if (props.pack.presence === 'outside') return 'outside your library'
	if (props.pack.presence === 'unchecked') return 'not in the scanned folder'
	return null
})

/**
 * The same offline-safe coloured initial AddonThumb uses for addons without an
 * icon — derived from the name so a pack keeps its colour between launches.
 * It covers a custom instance with no artwork at all, a CurseForge pack whose
 * thumbnail has not arrived yet, and one whose fetch failed. All three look the
 * same on purpose: none of them is a problem the user has to act on.
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

const relative = (iso: string): string | null =>
{
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
 * The most recent thing CEMM did with this pack, whatever it was. This used to
 * be chosen by the app's mode, which meant a pack could lead the grid — sorted
 * there by a history the card then declined to mention — with nothing saying
 * why. The card has no mode any more, so it simply reports the latest fact.
 */
const historyLabel = computed(() =>
{
	const entry = props.pack.history
	if (entry === null) return null

	const events: Array<[string, string | undefined]> = [
		['Published', entry.lastPublishedAt],
		['Updated', entry.lastInstalledAt],
		['Opened', entry.lastOpenedAt]
	]

	let latest: { verb: string, at: number, iso: string } | null = null
	for (const [verb, iso] of events)
	{
		if (iso === undefined) continue
		const at = Date.parse(iso)
		if (!Number.isFinite(at)) continue
		if (latest === null || at > latest.at) latest = { verb, at, iso }
	}
	if (latest === null) return null

	const when = relative(latest.iso)
	return when === null ? null : `${latest.verb} ${when}`
})
</script>
