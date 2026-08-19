# Design

<!-- impeccable:design-schema 1 -->

Recorded from the built interface, not from intent. Where this document and the
code disagree, the code is right and this file is stale.

## Direction

CEMM is a **modpack client** — a peer to the tools its users already have open,
not a themed novelty and not an anonymous grey dev tool.

Craft bar, set explicitly by the user: **CurseForge, Modrinth, Steam**. Those are
the applications CEMM sits beside on the same desktop, and two of them are used
for this exact task. The shell, the row grammar and the interaction vocabulary
are drawn from that family.

Four earlier directions were dealt and rejected before this one (departures
board, jet-age ticket wallet — built and discarded, conservation survey, esports
draft). The rejection that mattered was **"too costume-y"**: deriving the visual
world from a physical object produced fancy dress on a utility. This direction is
derived from an *interface architecture* instead.

Seed key `1a1b65bc`. The direction contract lives in the opening comment of
`app/app.vue` and ships in the bundle.

## Platform

Tauri 2 desktop app for Windows and Linux, Nuxt 4 + Vue 3 webview, `ssr: false`.
Window is frameless (`decorations: false`) with app-drawn chrome.

## Colour

Two daisyUI themes in `app/assets/css/main.css`, and **nothing else defines an
interface colour**. There is no parallel `:root` token layer; that duplication
was the root cause of the previous build's inconsistency and it is gone.

- `cemm-dark` — `prefersdark: true`. Ground `#141519`, panels `#1B1D23`, raised
  `#22252D`, content `#ECEEF3`.
- `cemm-light` — `default: true`. Ground `#FFFFFF`, panels `#F5F6F8`, raised
  `#E3E5EA`, content `#14161B`.

**Accent is violet** (`#7C6CF6` dark, `#5B4BE0` light). This is deliberate and
load-bearing: green belongs to Modrinth and orange to CurseForge, so taking
either would read as a clone. Violet also leaves the status hues unambiguous.

Status colours mean exactly one thing each and are never used decoratively:

| Token | Meaning |
| --- | --- |
| `success` | Added — a new addon arriving |
| `info` | Updated — an addon replaced with a newer version |
| `error` | Deleted / excluded — something being removed |
| `primary` | Brand, primary actions, active navigation, selected filters |

Every status is also carried by a **word** (`New`, `Update`, `Delete`, `Same`),
so the diff reads correctly in greyscale and to anyone who cannot separate hues.

Radii are `0.5rem` across selector, field and box. `--depth: 0`, `--noise: 0` —
flat, like all three reference products.

## Type

- **Archivo** (variable, weight 100–900 *and* width 62–125%) for everything.
- **JetBrains Mono** for versions, filenames and update codes.

Both are bundled under `public/fonts` and declared with `@font-face`. Nothing is
fetched at runtime: CEMM must render text on a cold offline launch, which a CDN
webfont cannot guarantee. Icons were already localised for the same reason.

Row names 13.5px, secondary lines 11–12px, headings 24px semibold with tight
tracking. `tabular-nums` on every figure that sits in a column.

## Structure

```
┌──────────────────────────────────────────────┐
│ TitleBar   mark · theme · − □ ×               │ 36px, drag region
├────┬─────────────────────────────────────────┤
│Rail│ heading / lede                           │ 54px rail, 200px expanded
│    │ context bar — the per-use input          │
│    │ ─────────────────────────────────────── │
│    │ scrolling work area (or empty state)     │
│    │ ─────────────────────────────────────── │
│    │ pinned action bar                        │
└────┴─────────────────────────────────────────┘
```

`WorkspacePage` owns this frame; both counters fill its slots. The action bar is
pinned so the commit button never scrolls out of reach on a long manifest.

### One screen per flow, not a wizard

**Configuration is not a step.** An earlier build paginated each flow into a
stepper, but most of those steps were settings that never change between uses —
the GitHub repository is identical for a whole group forever, and the modpack
folder is set once. A returning player was walked through two screens of
unchanged configuration to reach the one field they needed, on every update.

So each flow is a single surface that grows:

| Zone | Player | Admin |
| --- | --- | --- |
| Context bar | Destination (name **and full path**), then the update code | Loaded instance, plus publish name as an editable chip |
| Work area | Empty state → the diff | Empty state → category/config panes |
| Action bar | Destination + acknowledgement + Install | Ship counts + Publish, then the resulting code in place |

**The install does not replace the screen it was launched from.** It used to:
pressing Install swapped the diff for a progress bar, so the thing the player
had just read and consented to disappeared the moment they acted on it, and came
back as a bare success panel with nothing to check it against. It now runs in a
modal `<dialog>` over the diff, which stays put and afterwards reads as the
record of what was applied — with its copy moved to the past tense, because a
panel still saying "1 addon **will** be deleted" after the deletion is a surface
reporting something untrue. Dismissing the dialog returns to that diff; nothing
resets until the player asks for another update.

Being modal is also a safety property, not just a layout one: the rail was
clickable mid-install, which unmounted the panel while the Rust side carried on
writing. Escape and backdrop dismissal are refused while files are being
written, and the failure state does not claim the modpack is untouched — an
install writes file by file, so a failure part-way through leaves what it had
already done.

The context bar **scrolls with the content**; it is not pinned. Pinning it cost
~150px permanently and made the diff scroll underneath it, partly hiding the
deletions panel — the most safety-critical thing on the screen — behind an input
the user had already finished with. What must survive scrolling lives in the
action bar instead, which is why the destination is echoed there: that is where
the irreversible button is.

The destination shows the folder name **and the full path**. Showing only the
basename was unsafe in practice — a real library holds both `FTB Evolution` and
`FTB Evolution (1)`, which rendered as two identical chips for two different
modpacks.

Anything stable moved to first-run setup (`pages/index.vue`, which now captures
mode, repository and folder) and stays editable inline and in Settings. There is
no `StepRail`. Upload progress lives in the action bar, which is what it always
was; install progress moved to its own dialog for the reason above.

There is **no landing interstitial**. First run asks once, then redirects to the
workspace forever after — but the screen is **reachable again** from
*Settings > Setup*, because the Install/Publish choice was made there and had no
other way back. Re-running it clears only the choice: the repository and folder
are carried in already filled, since the point is to reopen the decision, not to
wipe the configuration made alongside it. The screen says "Setup" rather than
"First use" once anything is configured.

Switching sides that way clears any loaded manifest, exactly as switching from
the rail does — a diff fetched as a player is not the admin's working set.

### The library is a third destination, not a landing screen

**Your packs** reads the local CurseForge library and lists it as a card grid.
It is a peer of Install and Publish in the rail, deliberately not the screen the
app opens on: the job is still to ship or receive an update, and a library you
pass through every launch is the interstitial this build already removed once.

Cards rather than the app's usual dense rows, because this is the one screen
whose job is **recognition** rather than reading — you are picking one modpack
out of forty, which is exactly what CurseForge's and Modrinth's own overviews
are for. It is the only place in CEMM that departs from the row grammar, and
that departure is the reason.

**History is expressed on the library, not as a second list.** A separate
"recently used" list would be empty at the one moment it is most needed — first
run. So packs CEMM has been used with sort to the front and carry a mark naming
the most recent thing it did with them (`Published 3d ago`, `Updated 3d ago`,
`Opened 3d ago`) — the latest fact, not the one the current mode cares about,
because a card that leads the grid on a history it then declines to mention is
the list keeping a secret. Everything else falls back to how recently CurseForge
says it was played, and a pack CEMM has used that the scan no longer finds stays
listed as `Missing` rather than vanishing. Only timestamps are kept; a
remembered update code sitting beside a pack that has since changed would be a
lie waiting to happen.

Every card states its **folder** as well as its name, for the reason the install
destination already does: in a real library the folder `All the Mods 10 - ATM10
(2)` holds a pack named `Aeronautics`, and `FTB Evolution (2)` holds one named
`FTB Evolution (1)`. A name-only card points at something the user cannot
identify.

Filters are CurseForge's own groups, read from its `groups.json`, so the pills
are the ones the user already organised their library with — plus `Used in
CEMM`, `Ungrouped`, and free-text search across name, folder, version and
loader.

**The library has no mode, and the card is not a control.** The first build made
it one big button whose meaning came from whichever counter you had arrived
from — the identical click installed or published depending on state set on
another screen, and reaching the other behaviour meant leaving, switching
counter, and coming back through a link. That is a mode error, and a bad one:
two destructive-adjacent actions behind one indistinguishable gesture.

Every card now names **both** actions outright, and picking one is what sets the
counter rather than the counter deciding what the pick meant. Two is few enough
to show rather than hide: a menu is for actions that are many or secondary, and
these are neither — they are the only two things a pack is for. The buttons
carry the rail's own icons, so a control and the destination it leads to look
like the same thing.

Install asks for the update code **on the card**, so choosing a pack and saying
which update to put on it stay one gesture. Publish loads the instance and goes
straight to the diff. Switching sides this way clears any loaded manifest, for
the same reason the rail does — otherwise an admin would end up diffing against
a manifest the player had downloaded.

No virtualisation. The author's library is 36 packs and the scan behind it takes
~70ms warm (~400ms cold), because the Rust side deserializes a header struct
that counts `installedAddons` without building them. If someone turns up with
hundreds of instances, the grid is where to revisit.

**Discovery is allowed to fail.** CurseForge ships no official Linux build, so
"no library found" is an ordinary outcome with a way forward rather than an
error, the manual folder picker stays a first-class path, and Settings carries
an override. A failed scan that still leaves history packs on screen shows a
strip above the list rather than an empty state over the top of it — the screen
must not report "no library" while its own footer counts packs.

### One control per destination

Mode switching lives **only** in the icon rail. The title bar briefly carried a
second segmented control doing the same job 40px away; one destination gets one
control.

### The rail has two widths

Compact (54px, the default) is icon-only with tooltips and is why the rail
exists at all — it costs 54px instead of the 200px a labelled sidebar takes from
the diff. Expanded (200px) names every destination in place, for anyone who
would rather read than recognise an icon. The choice persists.

The icon box is a fixed 38px at the **start** of every row in both states, so
widening the rail never moves an icon: the panel grows and the labels arrive
beside them. Tooltips are dropped the moment the labels are visible, or each row
would state its name twice.

### Settings is a list, not a filing cabinet

Settings held four tabs (Repository, Appearance, Updates, About) covering **four
actual settings**. Updates was one version string and one button; About is not a
setting at all. Tabs earn their navigation cost when a pane holds more than you
can take in at once. Here they made you click to discover there was almost
nothing behind them, and every pane but the first read as mostly empty.

It is now **one scrolling page** in a 768px column: Repository (with the token),
Appearance, Accessibility, Setup, and About with the update check folded into
it. Each group is a
bordered panel of `SettingsRow`s, one setting per row, name and explanation on
the left and the control on the right so every control lands on the same
vertical axis. Below ~700px the rows stack.

The uppercase tracked legends (`REPOSITORY`, `ACCESS TOKEN`) are gone. They were
leftover eyebrow styling from the discarded wallet direction, and three of them
stacked above three sections is a templated rhythm rather than hierarchy.
Helper copy that repeated the field's own value went with them: the repository
hint read "for example YassaaaTU/cemm-updates" directly beneath a field showing
exactly that.

## Components

| Component | Role |
| --- | --- |
| `TitleBar` | Frameless chrome: brand, theme menu, real window controls |
| `IconRail` | Destination rail in two widths, sole owner of mode switching |
| `WorkspacePage` | Heading / context / scroll / pinned actions frame |
| `EmptyState` | Resting state carrying its own next action |
| `BrandMark` | The identity glyph, filled, inheriting `currentColor` |
| `SettingsGroup` | Titled panel of setting rows; `as` lets it be a `<form>` |
| `SettingsRow` | One setting: label and description left, control right |
| `AddonTable` | The dense list — thumbnail, name + filename, version + note, action slot |
| `AddonThumb` | Addon icon with an offline-safe coloured initial fallback |
| `StatusChip` | Word-first status label |
| `UpdatePreview` | The diff: four tallies, deletions panel, filtered incoming list |
| `InstallProgressDialog` | The install, run over the diff rather than instead of it |
| `PackCard` | One modpack in the library: icon, name, version, group, CEMM history |
| `PackUpdateDialog` | Asks for the update code against the pack that was clicked |

### The row grammar

Taken directly from CurseForge and Modrinth: 32px thumbnail, bold name with the
real `.jar` filename beneath, version with context under it (`from 15.2.0.27`,
`new install`, `removed from disk`), then the action. This is why `Addon`'s
`fileNameOnDisk` and `thumbnailUrl` are surfaced rather than hidden.

Admin exclusion is a **toggle**, matching how both reference apps enable and
disable a mod. Off means the addon stays on the maintainer's machine and is left
out of the upload; the row strikes through, dims and tints.

## Identity

The mark is an **isometric box with its lid lifted clear of the body**. CEMM
never ships a whole modpack, only the difference to one you already have, so the
glyph is a container receiving a part rather than a closed cube.

Faces are filled, not outlined. The mark is drawn at 17px in the title bar and
16px in the Windows taskbar, and at that size the previous 1.4px stroke closed
up into a smudge.

One geometry, two files, both in `public/brand`:

| File | Use |
| --- | --- |
| `cemm-mark.svg` | The glyph alone, `currentColor`, no tile — README, docs, anywhere the surface colour is not ours |
| `cemm-icon.svg` | The same glyph on the violet tile, 1024px — the **source** for the whole Tauri icon set |

`bunx tauri icon public/brand/cemm-icon.svg` regenerates `src-tauri/icons` from
that one file, so the window icon and the mark inside the window can never drift
apart. The generated Android and iOS sets are deleted: this is a desktop app and
`tauri.conf.json` references neither.

The tile is violet for the same reason the accent is — green is Modrinth's and
orange is CurseForge's, and the icon has to sit beside both in a taskbar reading
as a peer rather than a clone.

## Safety

The diff is the product's most consequential screen and its treatment is fixed:

1. Deletions get their **own bordered panel, above everything else**, always
   expanded, never behind a filter or tab.
2. Deleted addons are **named individually**, not counted. "3 addons will be
   removed" is not enough information to consent to deleting files.
3. The commit button is `btn-error`, not `btn-primary`, when the update deletes.
4. The acknowledgement checkbox gates the button, and **resets on every fetch** —
   consent to one diff is not consent to a different one.

## Micro-interactions

**Every interactive control shows a pointer.** Tailwind v4's Preflight dropped
`cursor: pointer` from `<button>`. daisyUI's `.btn` puts it back, so styled
buttons were fine and the gap was invisible — but every *custom-styled* control
in the app was silently on `cursor: default`: the three window controls, the
rail's collapse toggle, the admin category pills, the diff filter pills, the
rename button and the theme menu items. Any bare `<button>` added from here on
needs `cursor-pointer` explicitly; there is no global rule doing it.

**A native `<button>` centres its own content**, whatever its `display` says.
That is what made the two setup cards look crooked: with one description running
to three lines and the other to two, the shorter card's icon sat 11.4px lower
and its call to action 11.4px higher — half a line-height — even though both
cards were the same height and the same top. The cards were not unequal, they
were anchored differently.

The fix is layout, not copy: the card is an explicit `flex flex-col`, which
overrides the centring and tops the icon out, and `mt-auto` on the call to
action pins it to the floor. The description then floats between two fixed
anchors and its length stops mattering — verified holding at four lines against
two. Matching the copy would only have hidden the bug until the next edit.

## Motion

One language: `--ease-out-quick` for entrances, `--ease-standard` for state.
Everything is 150–220ms.

Motion is expressed as **Vue `<Transition>` classes built from Tailwind
utilities** — no keyframes are authored, which keeps the stylesheet inside the
token boundary. `composables/useMotion.ts` combines the OS
`prefers-reduced-motion` setting with the in-app switch and returns empty class
strings when motion is unwelcome, so a single gate covers both controls.

What moves, and why:

- **Step panes** fade and slide 8px on change — preventing a jarring swap.
- **Status alerts** fade up on enter, fade on leave — an error must not blink
  into existence.
- **Install progress** is eased toward its target in `useSmoothProgress`,
  because Tauri delivers it in discrete jumps that otherwise read as a broken bar.
- **Excluded rows** transition background and opacity over 150ms.

- **The rail** transitions its width over 220ms on `--ease-out-quick`. Labels
  fade in on a 90ms delay so text never appears in a gap too narrow to hold it,
  and leave immediately on collapse so it is never clipped mid-word. Icons hold
  position throughout.

What deliberately does **not** move: the diff tallies (data being read before a
destructive action must never animate), the theme switch, and addon rows
(removed after review — a virtualised list of 300 does not need a cascade).

## State

Every surface has a resting, busy, error and finished state, and the rules that
keep them from lying to the user:

- **A cleared surface goes fully back to resting.** Clearing a finished install
  used to leave the "Update installed" banner above an empty code field — the
  app reporting success for something the user had just cleared away. Clearing
  and starting over are now the same reset.
- **A filter belongs to the list it was typed against.** `AddonTable` is reused
  across the admin's categories rather than remounted, so a term typed in Mods
  survived into Data packs and hid a list that was not actually empty behind
  "Nothing matches". The search resets when the dataset does.
- **Cancelling is not failing.** Backing out of a native file dialog produced a
  six-second warning toast reporting that nothing had happened. It now returns
  quietly; only real failures notify.
- **Consent resets with the thing consented to** — the deletion acknowledgement
  clears on every fetch, but survives a *failed* install, because the diff on
  screen is still the one that was agreed to.
- Switching counters destroys the panel, so no admin state can leak into the
  player's screen or vice versa.
- **What was true before the action is not true after it.** The diff survives an
  install now, so every sentence on it that described something about to happen
  has to stop claiming that once it has — deletions in the past tense, the lede
  saying the update has been applied, the action bar offering another rather
  than the same one again.
- **A failure only empties what it actually emptied.** The pack library keeps
  showing packs CEMM knows from its own history when a scan fails, with a strip
  saying why, rather than an empty state contradicting a footer that is still
  counting them.
- **An action's meaning never comes from somewhere else on screen.** The pack
  library's cards used to install or publish depending on which counter you had
  come from, so the same click did two different things with nothing on the card
  saying which. Where a surface offers more than one thing to do, it names them.
- **"Missing" is only trusted when the scan was.** A pack absent from a failed
  scan, or from one that found no library at all, is not offered for removal —
  a wrong instances folder would otherwise make one click discard the history of
  every pack at once.

The faintest text tier (`base-content/45`) was raised to `/60`. At 12px on
`base-200` it was the one step in the muted scale that was hard to read in both
themes rather than merely quiet, and it carried real labels — "Installing to",
"Update code", and every on-disk filename in the table.

## Notifications

Outcomes are **toasts** (`vue-sonner`, bottom-right, offset above the action
bar), wrapped by `composables/useNotify.ts`. They replaced inline alert bars that
pushed content down on appearance and had to be dismissed by hand.

The rule that matters: **progress commentary is not a notification.** The Tauri
install and upload paths call their status callback repeatedly while work runs,
so each panel routes in-flight `info` messages to inline text beside the progress
bar and sends only outcomes to `useNotify`. Without that split, one install would
fire dozens of toasts.

Sonner owns the toast surface — `richColors` derives the semantic background per
type and follows the `theme` prop. Only radius and typography are overridden;
overriding the background would fight it and flatten success and error into
identical grey cards. `vue-sonner` is pinned in `optimizeDeps.include` so
`toast()` and `<Toaster/>` resolve to one module instance; without it Vite
prebundles them separately and toasts vanish into a store nothing renders.

## Accessibility

### Interface scale

One setting, five steps — 90 / 100 / 110 / 125 / 150% — and it resizes the
**whole shell**, not just the type: the rail, the title bar, the icon boxes, the
rows and the controls all move together.

It works by setting the **root font size**, which is why every length in the
interface is authored in `rem`. The five hard-coded pixel utilities that
survived (`[38px]` icon boxes, the `[54px]`/`[200px]` rail, the `[17px]` mark,
the `[3px]` active marker) were converted for this; there are now **no `px`
utilities anywhere in `app/`**, and that is a rule to keep, not a coincidence.

The value is a **percentage**, never a pixel size, so it multiplies whatever the
OS and webview already decided. Someone who has enlarged their system font keeps
that and gets 110% *of their size*. At 100% the inline style is removed
altogether rather than written as `100%`, on the same principle as `data-theme`:
absent means "whatever the machine already agreed".

Scaling the root rather than applying a transform or `zoom` means nothing blurs
and hit targets stay exactly where they are drawn.

**Ctrl and `+` / `−` / `0`** step and reset it, because this is a desktop app and
that is the shortcut every other window on the machine uses for this. The
handler reads `event.code`, so it lands on the physical key on any layout, and
accepts the numpad. Tauri leaves `zoomHotkeysEnabled` off, so there is nothing
to fight over.

Persisted values are normalised on read: a step this build does not have falls
back to 100% and is written back, because an unrecognised number applied to the
root font size would scale the app to a size no control could express.

### The rest

Binding, and carried over from the previous build's audit:

- Single `<main>` landmark with a skip-link target (`F-P2-8`).
- `aria-current` on navigation and the active step.
- `role="alert"` on status regions, `role="status"` on copy confirmation.
- Status never signalled by colour alone — every chip carries its word.
- Per-row toggles get a wrapping `<label>` naming the specific addon.
- Reduced motion honoured from both the OS and the in-app switch. The switch
  lives under **Accessibility**, not Appearance — it is the same control the OS
  exposes there, not a matter of taste.
- Light-theme status colours are darkened specifically to hold contrast against
  white surfaces, where the dark theme's brighter values would fail.

## Constraints

- **DaisyUI 5 is the component layer.** Non-negotiable.
- Authored CSS is limited to imports, theme/plugin directives, `@theme` tokens
  and `@font-face`. No component, layout, animation or interaction selectors.
- Offline-first. Two network exceptions, both explicit and bounded: GitHub, and
  CurseForge pack artwork. The second is fetched only from `media.forgecdn.net`
  over https, size-capped, and written to the app cache directory, so a given
  pack is fetched **once ever** and every later launch — including offline ones
  — is served from disk by a scan that never touches the network. A pack whose
  artwork has not arrived shows its coloured initial, which is also what a
  failed fetch leaves behind.
- `data-theme` is written **only** for an explicit user override, so `system`
  resolves in pure CSS with no flash. This retires the `F-P2-6` hydration trap.
