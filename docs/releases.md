# Releases: one profile, kept up to date

A legacy CEMM update is a list of addons plus whichever config files the admin
ticked. A **release** is a statement of the whole pack: every addon by exact
CurseForge file and hash, every pack file by hash, the settings the pack
requires, and the earlier content it is entitled to remove. Players import the
pack once and then apply release codes to that same profile.

Both kinds of code go in the same field. CEMM works out which one it was given.

- [For players](#for-players)
- [For the admin](#for-the-admin)
- [Upgrading to a new official pack release](#upgrading-to-a-new-official-pack-release)
- [Backups and recovery](#backups-and-recovery)
- [What CEMM never touches](#what-cemm-never-touches)
- [Things that still need a person](#things-that-still-need-a-person)
- [`cemm-pack.json` reference](#cemm-packjson-reference)

## For players

**Once.** Import the pack's export into CurseForge as its own profile. Do not
apply a release to the official pack's own profile: CurseForge keeps that one
updated itself and would overwrite the custom pack. CEMM refuses to, unless the
pack's admin has explicitly allowed it — and then tells you never to let
CurseForge update or repair that profile.

**Every update.**

1. Close Minecraft.
2. In CEMM, choose **Install** on the profile and paste the code.
3. Read the review. It lists what is added, replaced and removed, which
   required settings change, and what in your profile is yours and stays.
4. If it lists a file that needs your decision, you changed that file
   yourself. Tick it to let the release replace it (your version is kept in
   the backup), or cancel.
5. **Install update.** Then restart CurseForge if it was open, and back up your
   world before the first launch.

What to expect:

- The first time, CEMM *adopts* the profile: it checks every file against the
  release and takes ownership of the ones that match exactly. An imported
  profile downloads nothing.
- Skipping releases is fine. Any release gets you to the same state from any
  earlier one.
- Applying the same code twice does nothing the second time, except repair a
  file that went missing or changed.
- Mods you added yourself are listed as not part of the pack and left alone.
- Your keybinds and other settings in a file the pack also needs a value in
  are kept. Only the listed values change.

If the review says the update cannot be installed, nothing has been changed.
Each reason says what to do. The common ones:

| It says | Do this |
| --- | --- |
| Minecraft is running / a file is open | Close the game completely, then **Check again**. |
| This profile is the official pack itself | Import the custom pack's export as its own profile and use that. |
| Only N of this release's addons are here | Wrong profile, or not an import of this pack. |
| Needs the mod loader X | Set that modloader version in the profile's options in CurseForge, then apply again. |
| X cannot be downloaded for you | Download that exact file from the link given, put it in the named folder, **Check again**. |
| A settings file cannot be changed safely | Set the one value named by hand, then **Check again**. |
| This profile is on a newer release | CEMM does not go backwards. See [recovery](#backups-and-recovery). |
| This update was published for a newer CEMM | Update CEMM. |

A full code looks like `pack/release@` followed by 40 characters. The part
after `@` pins it to one exact publication. The short form without it works
too and is pinned when you fetch it.

## For the admin

### One-time setup

Put a `cemm-pack.json` in the profile root (start from
[`docs/examples/cemm-pack.fusion.json`](examples/cemm-pack.fusion.json)).
It is what turns the profile into a release line: with it, **Publish** opens on
a **Release** pane. Without it, publishing is the legacy flow, unchanged.

Decide what gets hosted. Everything under `content` is inventoried, hashed and
owned on players' machines. Whether its bytes are *uploaded* is the
`distribute` flag per path, and the repository is public: a code is not
access control. Content you may not redistribute can be inventoried with
`"distribute": false` — it is still verified, but CEMM cannot deliver or repair
it, and it can never change in a later release without being hosted.

### Publishing a release

1. Close Minecraft and let CurseForge finish whatever it is doing.
2. **Publish** on the profile, **Release** pane. Enter a release id (never
   reused; a date with a counter works), the previous release's code, and
   notes.
3. **Check release.** This hashes the whole profile and reports everything
   that blocks publication. Typical findings:
   - a jar in `mods/` that CurseForge has no record of and the spec does not
     mention — list it under `content` or `local_only`;
   - a jar that is not the file its CurseForge record describes — let
     CurseForge rescan, then check again;
   - a pin that does not match the installed file;
   - a required setting your own profile does not have;
   - a dependency the previous release had and this one drops;
   - merge-conflict markers left in a script.
4. **Publish release.** One commit, one branch move: the release appears
   whole or not at all. Only files this release introduces are uploaded.
5. Copy the code and **Save receipt** somewhere outside the profile.

The first release uploads every hosted file, one request each; for a large
pack that is several thousand. When GitHub asks CEMM to slow down, the publish
waits as long as it is told and carries on — the progress line says so. If it
is still being limited after about forty minutes it stops with nothing
published, and can simply be run again later. Routine releases upload only
what changed.

**Verifying a rebuilt profile.** Paste a release's code into **Install** on any
copy of the pack. A copy that matches reports nothing to change.

**Checking an export before handing it out.** Export the pack from CurseForge,
import that export as a throwaway profile, and paste the release's code into
**Install** on it. The review is the check: anything it would add or download
is something the export left out — a local fix jar that was not ticked, a
folder of scripts — and "already matches the release" means the export is
complete. Delete the throwaway profile afterwards.

### What old clients do

A release folder also contains a `cemm-manifest.json` that is deliberately not
a manifest. A CEMM older than 2.2.0 fetches it, fails to read it before
anything is previewed or written, and shows the reason: *"This update needs
CEMM 2.2.0 or newer."* Tell players to update CEMM before handing out the
first release code.

### Legacy codes

Still work. On a profile that follows a release line:

- a legacy **config-only** code applies as before, removes no addon, and is
  recorded on top of the release. If it overwrote a pack file, the next
  release review shows that file as changed;
- a legacy **full** code is refused. It is an addon list that knows nothing
  about what the release installed.

## Upgrading to a new official pack release

CEMM applies the result of an upgrade. It does not perform the merge.

Keep four states, all outside players' profiles:

- **B** — the official release the pack is currently built on, untouched.
- **F** — the pack as last released.
- **N** — the new official release, a clean download.
- **R** — the candidate next release, built from N plus the pack's changes.

Keep the original archives and their hashes. A folder that has been launched
contains generated settings and is not a clean baseline.

1. Freeze F. Publish or discard any unreleased changes first.
2. Install or extract N somewhere separate. Never use CurseForge's *Change
   Version* on the live pack profile as the merge.
3. **Publish → Release → Review an official pack upgrade.** Point it at B and
   N (installed profiles, or extracted pack archives with `manifest.json` and
   `overrides/`). It compares the three on the paths and addons your
   `cemm-pack.json` names and reports, per item:

   | The pack vs B | N vs B | Report says |
   | --- | --- | --- |
   | unchanged | changed | take the official change, including a deletion |
   | changed | unchanged | keep the pack's, including an intentional deletion |
   | same result on both sides | | keep one copy |
   | both changed, differently | | **conflict** — you decide |

   Also reported as conflicts: delete-versus-modify, a file added on both
   sides with different content, two paths differing only by letter case, and
   any official change to a **pinned** addon. Listed for review rather than
   decided: a library the official pack dropped that a custom addon still
   requires, a custom addon the official pack now ships itself, and every
   local fix jar.
4. Build R in a working copy, following the report. Resolve every conflict by
   hand. A clean text merge is not proof of a working recipe or quest; JS,
   SNBT, TOML and binaries need a look.
5. Update `base` in R's `cemm-pack.json` to N's exact project and file id.
6. Rebuild and test the matching server from N's official server files plus
   the pack's server-side changes. Never copy the client `mods` folder into
   it. Name the result in `server_release`.
7. **Check release** on R with F's code as the previous release. Unfinished
   merges, dropped dependencies, moved pins and unaccounted files block it.
8. Test R: dedicated server boot and save, real client join, the pack's own
   checklist. Then publish.

A Minecraft or mod loader change is not something a release can install.
Players are stopped with instructions; a major Minecraft change needs a
deliberate profile migration.

## Backups and recovery

**Every update that changes anything** keeps what it replaced in
`.cemm/backups/<time>-<release>/` inside the profile: `files/` mirrors the
profile, and `index.json` lists what was replaced and what was newly created.
The three most recent are kept.

To undo an update by hand, with Minecraft and CurseForge closed: copy
everything under that backup's `files/` back over the profile and delete the
paths `index.json` lists under `created`. CEMM's own record and CurseForge's
are part of the backup, so that puts both back as well.

**An interrupted update** (crash, power loss) is rolled back automatically the
next time the profile is reviewed or updated, and the review says so.

**Worlds are separate.** Once the game has saved a world on a new release,
putting the old mods back does not make that world safe on them. Back up
worlds before the first launch on a release, and if you go back, restore the
matching world backup together with the profile backup. CEMM never reads,
uploads, replaces or deletes worlds.

## What CEMM never touches

Enforced by the installer, whatever a release lists, and refused at publish
time on the admin side:

- `saves/`, world backups, `screenshots/`, map data, `local/`
- `options.txt` and the other personal option files, `servers.dat`
- `minecraftinstance.json` as a whole, launcher profiles and accounts
- logs, crash reports, generated `*.snbt_merged` fragments
- `server.properties`, `eula.txt`, ops and whitelist files
- anything that looks like a credential or key

CEMM does edit CurseForge's `minecraftinstance.json`, narrowly: for an addon
it replaced it updates the file id, name, size, fingerprint and enabled state
on the entry that already exists; it removes the entry of an addon the release
retired; and it sets the lock on pinned addons. Every other field — profile
name, memory, Java arguments, anything it does not recognise — is written
back exactly as found. It never copies the admin's file to anyone.

## Things that still need a person

- **Restart CurseForge** after an update if it was open, so it rereads the
  profile. CEMM detects CurseForge writing to the profile *during* an update
  and stops before changing anything; it cannot stop CurseForge afterwards.
- **Check the pin** in CurseForge after the first release: the pinned addon
  should show as locked. If CurseForge had no entry for it, CEMM says so and
  it has to be locked by hand once the entry appears.
- **New mods** CEMM installs have no CurseForge entry until CurseForge
  recognises the files on its next scan.
- **Back up worlds.** See above.

## `cemm-pack.json` reference

| Key | Meaning |
| --- | --- |
| `lineage` | The pack's permanent identity: lowercase letters, digits, single dashes. First half of every code. Renaming the profile does not change it. |
| `name` | What players see. |
| `role` | `client` (default) or `server`. CEMM installs client releases only. |
| `base` | `project_id`, `file_id`, `name` of the exact official pack file this is built on. |
| `minecraft`, `loader` | Optional. Read from the profile when left out; checked against it when given. |
| `server_release` | `name`, optional `sha256`, optional `notes`. Shown to players after an update. |
| `adoption.min_project_overlap_percent` | How much of the release's addon list must already be in a profile CEMM has never seen. Default 90. |
| `adoption.allow_official_profile` | Whether a profile CurseForge still links to the official pack may be adopted. Default `false`. Switch on only if your players' imports carry that link. |
| `supported_from` | Release ids an upgrade is allowed from. Empty means any earlier release. |
| `pins` | `project_id`, optional `file_id`, `reason`. The addon is locked in each player's CurseForge. |
| `exclude_addons` | CurseForge project ids in your profile that are not part of the pack. |
| `content` | Paths that are part of the pack. Each: `path` (file or folder), `policy`, `role`, `distribute`, `exclude`. |
| `local_only` | Glob patterns for files in the addon folders that are deliberately not shipped. |
| `settings` | Required values inside JSON files players otherwise own: `path`, `pointer` (JSON pointer), `value`, `reason`. |
| `manual_steps`, `known_issues` | Shown to players. |

`content[].policy`:

- `managed` (default) — the release's content. Must match; replaced when it
  changes; removed when a later release drops it. A copy the player edited is
  a decision for the player, never a silent overwrite or deletion.
- `default` — a starting point for something personal. Installed when
  missing; moved to a new default while the player has not touched it; left
  alone, and recorded as theirs, once they have.

A file with entries under `settings` should be published as `default`. It is
then a shared file: the pack owns the listed values and nothing else in it.

`content[].role` is `both` (default), `client` or `server`. Server-only content
is not installed into a client profile.

Unknown keys are an error, so a typo cannot quietly publish a pack with a
section missing.
