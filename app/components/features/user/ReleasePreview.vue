<template>
  <div class="space-y-4">
    <!-- Which pack, which release, and where this profile is coming from.
         A release names its own identity, so this is stated rather than
         inferred from a folder name. -->
    <div class="rounded-box border border-base-300 bg-base-200 px-4 py-3.5">
      <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
        <h3 class="text-[0.9375rem] font-semibold">
          {{ release.name }}
        </h3>
        <span class="font-mono text-xs text-base-content/60">{{ release.release_id }}</span>
        <StatusChip
          :tone="applied ? 'new' : 'updated'"
          :label="modeLabel"
        />
      </div>
      <p class="mt-1 text-xs text-base-content/65">
        {{ modeSentence }}
      </p>
      <dl class="mt-2.5 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-xs">
        <dt class="text-base-content/50">
          Built on
        </dt>
        <dd class="min-w-0 truncate">
          {{ baseLabel }} · Minecraft {{ release.minecraft }} · {{ release.loader }}
        </dd>
        <template v-if="serverRelease !== undefined">
          <dt class="text-base-content/50">
            Server
          </dt>
          <dd class="min-w-0 truncate">
            {{ serverRelease.name }}<template v-if="serverRelease.notes">
              — {{ serverRelease.notes }}
            </template>
          </dd>
        </template>
      </dl>
      <p
        v-if="release.notes"
        class="mt-2.5 border-t border-base-300 pt-2.5 text-sm/relaxed whitespace-pre-line text-base-content/80"
      >
        {{ release.notes }}
      </p>
    </div>

    <div
      v-if="plan.recovered_interrupted && !applied"
      role="status"
      class="alert alert-soft text-sm alert-warning"
    >
      <Icon
        name="mdi:backup-restore"
        size="1.1rem"
        aria-hidden="true"
      />
      <span>
        <strong>An earlier update here was interrupted.</strong>
        This profile has been put back to how it was before that update, and
        what you see below is against that restored state.
      </span>
    </div>

    <!-- Blockers lead: while one stands, nothing else on this screen can
         happen, and each says what to do about it. -->
    <div
      v-if="plan.blockers.length > 0 && !applied"
      class="overflow-hidden rounded-box border border-error/60"
    >
      <div class="flex items-start gap-2.5 bg-error/10 px-3.5 py-3">
        <span class="grid size-6 shrink-0 place-items-center rounded-md bg-error/20 text-error">
          <Icon
            name="mdi:block-helper"
            size="0.8125rem"
            aria-hidden="true"
          />
        </span>
        <div class="min-w-0">
          <p class="text-sm font-semibold text-error">
            This update cannot be installed here yet
          </p>
          <p class="mt-0.5 text-xs text-base-content/65">
            Nothing has been changed.
          </p>
        </div>
      </div>
      <ul>
        <li
          v-for="blocker in plan.blockers"
          :key="blocker.code + blocker.message"
          class="border-t border-base-300/60 px-3.5 py-2.5 text-sm/relaxed"
        >
          {{ blocker.message }}
        </li>
      </ul>
    </div>

    <!-- Decisions only the player can make. A required one stops the install
         until it is ticked; an optional one keeps their file unless ticked. -->
    <div
      v-if="plan.conflicts.length > 0 && !applied"
      class="overflow-hidden rounded-box border border-warning/60"
    >
      <div class="bg-warning/10 px-3.5 py-3">
        <p class="text-sm font-semibold">
          {{ plan.conflicts.length === 1 ? 'One file needs your decision' : `${plan.conflicts.length} files need your decision` }}
        </p>
        <p class="mt-0.5 text-xs text-base-content/65">
          You changed these yourself. Whatever you choose, your version is kept in the backup.
        </p>
      </div>
      <ul>
        <li
          v-for="conflict in plan.conflicts"
          :key="conflict.id"
          class="border-t border-base-300/60 px-3.5 py-2.5"
        >
          <label class="flex cursor-pointer items-start gap-2.5">
            <input
              type="checkbox"
              class="checkbox mt-0.5 checkbox-sm"
              :checked="approved.includes(conflict.id)"
              :disabled="locked"
              @change="toggleApproval(conflict.id)"
            />
            <span class="min-w-0">
              <span class="block font-mono text-xs break-all">{{ conflict.id }}</span>
              <span class="mt-0.5 block text-xs/relaxed text-base-content/70">{{ conflict.message }}</span>
              <span class="mt-1 block text-xs font-medium">
                {{ conflict.required ? 'Tick to replace it. The update cannot finish otherwise.' : 'Tick to take the pack’s version. Left unticked, yours stays.' }}
              </span>
            </span>
          </label>
        </li>
      </ul>
    </div>

    <div class="grid grid-cols-2 gap-2.5 sm:grid-cols-4">
      <div
        v-for="tally in tallies"
        :key="tally.label"
        class="rounded-box border border-base-300 bg-base-200 px-3.5 py-3"
      >
        <p class="text-xs text-base-content/50">
          {{ tally.label }}
        </p>
        <p
          class="mt-1.5 font-mono text-2xl leading-none font-bold tabular-nums"
          :class="tally.count > 0 ? tally.tone : 'text-base-content/25'"
        >
          {{ tally.count }}
        </p>
      </div>
    </div>

    <p
      v-if="plan.change_count === 0 && plan.blockers.length === 0 && !applied"
      class="rounded-box border border-success/40 bg-success/10 px-3.5 py-3 text-sm"
    >
      {{ plan.mode === 'adopt'
        ? 'This profile already matches the release. Installing records that, so later updates only fetch what changes.'
        : 'This profile already matches the release. There is nothing to download or remove.' }}
    </p>

    <AddonTable
      v-if="addonRows.length > 0"
      id="release-addons"
      :title="applied ? 'Addons changed' : 'Addon changes'"
      :rows="addonRows"
      :total="addonRows.length"
      :max-height="280"
      noun="addons"
      show-type
      empty-label="No addon changes."
    />

    <div
      v-if="fileRows.length > 0"
      class="overflow-hidden rounded-box border border-base-300 bg-base-200"
    >
      <div class="flex items-center gap-3 border-b border-base-300 px-3 py-2.5">
        <h3 class="text-sm font-semibold">
          Pack files
        </h3>
        <span class="font-mono text-xs text-base-content/50 tabular-nums">{{ fileRows.length }}</span>
        <span class="min-w-0 truncate text-xs text-base-content/55">
          Scripts, quests and settings the pack ships. {{ verifiedFiles }} more already match.
        </span>
      </div>
      <ul class="max-h-56 overflow-y-auto">
        <li
          v-for="file in fileRows"
          :key="file.path"
          class="flex items-center gap-2 border-b border-base-300/40 px-3 py-1.5 last:border-b-0"
        >
          <StatusChip
            :tone="file.tone"
            :label="file.label"
          />
          <span
            class="min-w-0 flex-1 truncate font-mono text-xs text-base-content/75"
            :title="file.path"
          >{{ file.path }}</span>
          <span
            v-if="file.localBinary"
            class="shrink-0 rounded-full border border-warning/50 px-2 py-0.5 text-[0.6875rem] text-warning"
            title="A mod file supplied by your admin, not downloaded from CurseForge."
          >Local mod</span>
          <span
            v-if="file.note"
            class="hidden max-w-[45%] shrink-0 truncate text-xs text-base-content/55 sm:block"
            :title="file.note"
          >{{ file.note }}</span>
        </li>
      </ul>
    </div>

    <div
      v-if="plan.settings.length > 0"
      class="overflow-hidden rounded-box border border-base-300 bg-base-200"
    >
      <div class="flex items-center gap-3 border-b border-base-300 px-3 py-2.5">
        <h3 class="text-sm font-semibold">
          Required settings
        </h3>
        <span class="min-w-0 truncate text-xs text-base-content/55">
          Only these values are set. The rest of each file stays yours.
        </span>
      </div>
      <ul>
        <li
          v-for="setting in plan.settings"
          :key="setting.path + setting.pointer"
          class="border-b border-base-300/40 px-3 py-2 last:border-b-0"
        >
          <div class="flex flex-wrap items-center gap-2">
            <StatusChip
              :tone="setting.op === 'apply' ? 'updated' : 'unchanged'"
              :label="setting.op === 'apply' ? (applied ? 'Changed' : 'Change') : 'Already set'"
            />
            <span class="font-mono text-xs break-all">{{ setting.path }}</span>
          </div>
          <p class="mt-1 font-mono text-xs text-base-content/70">
            {{ setting.pointer }}
            <template v-if="setting.op === 'apply'">
              : {{ setting.current ?? 'not set' }} → {{ setting.required }}
            </template>
            <template v-else>
              = {{ setting.required }}
            </template>
          </p>
          <p
            v-if="setting.reason"
            class="mt-0.5 text-xs text-base-content/55"
          >
            {{ setting.reason }}
          </p>
        </li>
      </ul>
    </div>

    <!-- What CEMM does to CurseForge's own record, said plainly because it is
         the part a player cannot see happen. -->
    <div
      v-if="launcherLines.length > 0"
      class="rounded-box border border-base-300 bg-base-200 px-3.5 py-3"
    >
      <h3 class="text-sm font-semibold">
        CurseForge
      </h3>
      <ul class="mt-1.5 space-y-1 text-xs/relaxed text-base-content/75">
        <li
          v-for="line in launcherLines"
          :key="line"
        >
          {{ line }}
        </li>
      </ul>
    </div>

    <div
      v-if="plan.extras.length > 0"
      class="rounded-box border border-base-300 bg-base-200 px-3.5 py-3"
    >
      <h3 class="text-sm font-semibold">
        {{ plan.extras.length === 1 ? 'One file here is not part of this pack' : `${plan.extras.length} files here are not part of this pack` }}
      </h3>
      <p class="mt-0.5 text-xs text-base-content/65">
        CEMM did not install {{ plan.extras.length === 1 ? 'it' : 'them' }}, so {{ plan.extras.length === 1 ? 'it is' : 'they are' }} left exactly as {{ plan.extras.length === 1 ? 'it is' : 'they are' }}.
      </p>
      <ul class="mt-2 max-h-28 space-y-0.5 overflow-y-auto font-mono text-xs text-base-content/70">
        <li
          v-for="extra in plan.extras"
          :key="extra.path"
          class="truncate"
          :title="extra.path"
        >
          {{ extra.label }}
        </li>
      </ul>
    </div>

    <div
      v-if="advisories.length > 0"
      class="rounded-box border border-base-300 bg-base-200 px-3.5 py-3"
    >
      <h3 class="text-sm font-semibold">
        {{ applied ? 'Before you play' : 'Good to know' }}
      </h3>
      <ul class="mt-1.5 list-disc space-y-1 pl-4 text-xs/relaxed text-base-content/75">
        <li
          v-for="advisory in advisories"
          :key="advisory"
        >
          {{ advisory }}
        </li>
      </ul>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { AddonRow } from '~/components/domains/addons/AddonTable.vue'
import type { StatusTone } from '~/components/shared/ui/StatusChip.vue'
import type { ApplyReport, InstallPlan, Release } from '~/types'

const props = withDefaults(
	defineProps<{
		release: Release
		plan: InstallPlan
		/** Set once the install has finished; the screen then reads as a record. */
		report?: ApplyReport | null
		/** An install in flight or finished — decisions stop taking input. */
		locked?: boolean
	}>(),
	{ report: null, locked: false }
)

/** Conflict ids the player has approved. */
const approved = defineModel<string[]>('approved', { default: () => [] })

const applied = computed(() => props.report !== null)

const toggleApproval = (id: string) =>
{
	approved.value = approved.value.includes(id)
		? approved.value.filter((existing) => existing !== id)
		: [...approved.value, id]
}

const serverRelease = computed(() => props.release.server_release)

const baseLabel = computed(() =>
	props.release.base.name.length > 0
		? props.release.base.name
		: `official pack file ${props.release.base.file_id}`
)

const modeLabel = computed(() =>
{
	if (applied.value) return 'Installed'
	if (props.plan.mode === 'adopt') return 'First update'
	if (props.plan.mode === 'reapply') return 'Check & repair'
	return 'Update'
})

const modeSentence = computed(() =>
{
	if (props.plan.mode === 'adopt')
	{
		return applied.value
			? 'CEMM now tracks this profile. Later updates only fetch what changes.'
			: 'CEMM has not updated this profile before. It checks what is here against the release and takes over only the files that match.'
	}
	if (props.plan.mode === 'reapply')
	{
		return `This profile is already on ${props.release.release_id}. Installing again checks it and repairs anything that drifted.`
	}
	return `From ${props.plan.from_release ?? 'an earlier release'} to ${props.release.release_id}.`
})

const countOps = <T extends { op: string }>(items: T[], ...ops: string[]) =>
	items.filter((item) => ops.includes(item.op)).length

const sum = (...values: number[]) => values.reduce((total, value) => total + value, 0)

const tallies = computed(() => [
	{
		label: 'Added',
		count: countOps(props.plan.addons, 'install', 'enable') + countOps(props.plan.files, 'add'),
		tone: 'text-success'
	},
	{
		label: 'Updated',
		count: sum(
			countOps(props.plan.addons, 'replace'),
			countOps(props.plan.files, 'replace', 'repair'),
			countOps(props.plan.settings, 'apply')
		),
		tone: 'text-info'
	},
	{
		label: 'Removed',
		count: countOps(props.plan.addons, 'retire', 'disable') + countOps(props.plan.files, 'retire'),
		tone: 'text-error'
	},
	{
		label: 'Already right',
		count: countOps(props.plan.addons, 'verified') + countOps(props.plan.files, 'verified'),
		tone: 'text-base-content/70'
	}
])

const ADDON_PRESENTATION: Record<string, { tone: StatusTone, label: string, done: string, struck?: boolean }> = {
	install: { tone: 'new', label: 'New', done: 'Installed' },
	replace: { tone: 'updated', label: 'Update', done: 'Updated' },
	enable: { tone: 'updated', label: 'Switch on', done: 'Switched on' },
	disable: { tone: 'excluded', label: 'Switch off', done: 'Switched off' },
	retire: { tone: 'removed', label: 'Delete', done: 'Deleted', struck: true }
}

/** Only what changes: a list of five hundred "same" rows hides the five that matter. */
const addonRows = computed<AddonRow[]>(() =>
	props.plan.addons
		.filter((action) => action.op !== 'verified')
		.map((action) =>
		{
			const presentation = ADDON_PRESENTATION[action.op] ?? { tone: 'unchanged' as const, label: action.op, done: action.op }
			return {
				key: `${action.project_id}-${action.op}-${action.file_name}`,
				name: action.name,
				// `note` is left off the wire when there is nothing to say.
				subtitle: (action.note ?? '').length > 0 ? (action.note ?? '') : action.file_name,
				version: action.file_name,
				versionNote: action.from_file !== undefined && action.from_file !== action.file_name
					? `from ${action.from_file}`
					: '',
				tone: presentation.tone,
				label: applied.value ? presentation.done : presentation.label,
				badge: action.pinned ? 'Pinned' : undefined,
				struck: presentation.struck === true,
				category: action.category
			}
		})
)

const FILE_PRESENTATION: Record<string, { tone: StatusTone, label: string, done: string }> = {
	add: { tone: 'new', label: 'New', done: 'Added' },
	replace: { tone: 'updated', label: 'Update', done: 'Updated' },
	repair: { tone: 'updated', label: 'Replace', done: 'Replaced' },
	retire: { tone: 'removed', label: 'Delete', done: 'Deleted' },
	keep_local: { tone: 'unchanged', label: 'Yours', done: 'Kept yours' }
}

const fileRows = computed(() =>
	props.plan.files
		.filter((file) => file.op !== 'verified' && file.op !== 'skip_role')
		.map((file) =>
		{
			const presentation = FILE_PRESENTATION[file.op] ?? { tone: 'unchanged' as const, label: file.op, done: file.op }
			// A conflict left unticked means the file stays as the player has it.
			const pending = file.op === 'repair' && !approved.value.includes(file.path)
			return {
				path: file.path,
				tone: pending ? 'unchanged' as const : presentation.tone,
				label: pending ? 'Needs you' : (applied.value ? presentation.done : presentation.label),
				note: file.note ?? '',
				localBinary: file.source === 'local_binary'
			}
		})
)

const verifiedFiles = computed(() => countOps(props.plan.files, 'verified'))

const launcherLines = computed(() =>
{
	const launcher = props.report?.launcher ?? props.plan.launcher
	const lines: string[] = []
	const tense = applied.value
	if (launcher.records_updated > 0)
	{
		lines.push(`${launcher.records_updated} entr${launcher.records_updated === 1 ? 'y' : 'ies'} in CurseForge's mod list ${tense ? 'now match' : 'will be brought in line with'} the installed files.`)
	}
	if (launcher.records_removed > 0)
	{
		lines.push(`${launcher.records_removed} entr${launcher.records_removed === 1 ? 'y' : 'ies'} for removed mods ${tense ? 'were' : 'will be'} taken off CurseForge's list.`)
	}
	for (const pin of launcher.pins)
	{
		if (pin.state === 'already_pinned') lines.push(`${pin.name} stays locked on ${pin.file_name}.`)
		else if (pin.state === 'will_pin') lines.push(`${pin.name} ${tense ? 'is now' : 'will be'} locked on ${pin.file_name}, so CurseForge does not update it.`)
		else lines.push(`${pin.name} must stay on ${pin.file_name}. CurseForge does not list it yet — lock it by hand once it appears.`)
	}
	if (lines.length > 0 && !tense)
	{
		lines.push('If CurseForge is open, restart it afterwards so it rereads this profile.')
	}
	return lines
})

/** Everything worth reading that is not a change: notes, steps, known issues. */
const advisories = computed(() =>
{
	const lines = [
		...props.plan.notes,
		...(props.report?.warnings ?? []),
		...props.release.manual_steps ?? [],
		...(props.release.known_issues ?? []).map((issue) => `Known issue: ${issue}`)
	]
	if (props.report?.backup_dir !== undefined)
	{
		lines.push(`The files this update replaced are kept in ${props.report.backup_dir} inside the profile.`)
	}
	if (props.report !== null && props.report.kept_local.length > 0)
	{
		lines.push(`Your own version was kept for: ${props.report.kept_local.join(', ')}.`)
	}
	return [...new Set(lines)]
})
</script>
