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
│Rail│ heading / lede                           │ 54px icon rail
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
no `StepRail`; install and upload progress live in the action bar, which is what
they always were.

There is **no landing interstitial**. First run asks once, then redirects to the
workspace forever after.

### One control per destination

Mode switching lives **only** in the icon rail. The title bar briefly carried a
second segmented control doing the same job 40px away; one destination gets one
control.

## Components

| Component | Role |
| --- | --- |
| `TitleBar` | Frameless chrome: brand, theme menu, real window controls |
| `IconRail` | 54px destination rail, sole owner of mode switching |
| `WorkspacePage` | Heading / context / scroll / pinned actions frame |
| `EmptyState` | Resting state carrying its own next action |
| `AddonTable` | The dense list — thumbnail, name + filename, version + note, action slot |
| `AddonThumb` | Addon icon with an offline-safe coloured initial fallback |
| `StatusChip` | Word-first status label |
| `UpdatePreview` | The diff: four tallies, deletions panel, filtered incoming list |

### The row grammar

Taken directly from CurseForge and Modrinth: 32px thumbnail, bold name with the
real `.jar` filename beneath, version with context under it (`from 15.2.0.27`,
`new install`, `removed from disk`), then the action. This is why `Addon`'s
`fileNameOnDisk` and `thumbnailUrl` are surfaced rather than hidden.

Admin exclusion is a **toggle**, matching how both reference apps enable and
disable a mod. Off means the addon stays on the maintainer's machine and is left
out of the upload; the row strikes through, dims and tints.

## Safety

The diff is the product's most consequential screen and its treatment is fixed:

1. Deletions get their **own bordered panel, above everything else**, always
   expanded, never behind a filter or tab.
2. Deleted addons are **named individually**, not counted. "3 addons will be
   removed" is not enough information to consent to deleting files.
3. The commit button is `btn-error`, not `btn-primary`, when the update deletes.
4. The acknowledgement checkbox gates the button, and **resets on every fetch** —
   consent to one diff is not consent to a different one.

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

What deliberately does **not** move: the diff tallies (data being read before a
destructive action must never animate), the theme switch, the icon rail beyond a
colour change, and addon rows (removed after review — a virtualised list of 300
does not need a cascade).

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

Binding, and carried over from the previous build's audit:

- Single `<main>` landmark with a skip-link target (`F-P2-8`).
- `aria-current` on navigation and the active step.
- `role="alert"` on status regions, `role="status"` on copy confirmation.
- Status never signalled by colour alone — every chip carries its word.
- Per-row toggles get a wrapping `<label>` naming the specific addon.
- Reduced motion honoured from both the OS and the in-app switch.
- Light-theme status colours are darkened specifically to hold contrast against
  white surfaces, where the dark theme's brighter values would fail.

## Constraints

- **DaisyUI 5 is the component layer.** Non-negotiable.
- Authored CSS is limited to imports, theme/plugin directives, `@theme` tokens
  and `@font-face`. No component, layout, animation or interaction selectors.
- Offline-first: no runtime network requests except explicit GitHub calls.
- `data-theme` is written **only** for an explicit user override, so `system`
  resolves in pure CSS with no flash. This retires the `F-P2-6` hydration trap.
