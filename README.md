# CEMM - ChillEcke Modpack Manager

CEMM (ChillEcke Modpack Manager) is a lightweight desktop application that makes it easier for you and your friends to play pre-existing CurseForge modpacks with custom modifications. Built with a Nuxt 4 frontend, a Tauri native shell, and a local Rust sidecar service.

CEMM opens on your CurseForge pack library. Picking what to do with a pack is what puts you on one of two sides — you are never asked to choose in the abstract:

- **Publish** (admin): Modify a downloaded CurseForge modpack (add, remove, or update addons and config files) → Generate an update code and upload changes to GitHub
- **Install** (user): Paste the update code from an admin → Review exactly what changes → Install it into the pack you chose

## Features
- **Modpack Modification**: Add, remove, or update mods/resourcepacks/shaderpacks/datapacks from CurseForge modpacks
- **Addon Exclusion**: Exclude specific mods or resourcepacks from the uploaded instance without removing them locally
- **Config File Distribution**: Share custom configuration files with your friends
- **Easy Sharing**: Generate update codes for simple modpack distribution
- **GitHub Integration**: Secure distribution via GitHub repositories
- **Automatic Installation**: Users can install modifications with a single update code
- **Recoverable Installation**: Add-ons, configs, and manifests finalize through a journaled transaction that rolls back interrupted or failed updates
- **Cross-Platform**: Developed and used on Windows. macOS and Linux builds are produced by CI, and the compiled sidecar's lifecycle is tested on all three, but the full app has never been run on macOS or Linux — treat those as untested.

## AI assistance

CEMM has had AI involved throughout. What changed at 2.0.0 is how, and the
commit history changes character at that boundary — this is here so anyone
reading it knows why.

Up to and including **v1.6.0**, the work was manual in the sense that mattered:
AI lived in a chat window. I was learning Rust, so I asked it questions and it
sometimes answered with code. Where that code was any good I read it, decided
whether it made sense, and brought it in myself. Nothing reached the repository
without going through my hands one piece at a time.

**From 2.0.0 onward the project is AI-assisted in a stronger sense.** The rewrite
and the work around it — the local sidecar architecture, the transactional
installer, and much of the surrounding refactoring — were written with AI working
directly in the codebase rather than beside it. I still set the scope, make the
architecture decisions, and decide what ships.

## Usage

### Admin Mode
1. Open the pack library and choose **Publish** on the modpack you want to distribute. CEMM reads its `minecraftinstance.json` and loads the addons it contains. (A pack outside the CurseForge library can be pointed at by hand instead.)
2. Review the addons by category — mods, resourcepacks, shaderpacks, datapacks.
3. **Exclude addons (optional)**: switch off the toggle on any row to keep it out of the published update.
   - It stays in your local modpack; it just is not distributed.
   - Useful for server-side mods, or anything platform-specific.
   - Addons already switched off in CurseForge start excluded. Switch one back on to publish it anyway.
4. Choose config files to distribute (optional) from the **Config files** tab.
5. **Publish update**. CEMM uploads to your GitHub repository and gives you an update code.
6. Share the update code.

### User Mode
1. Set your GitHub repository, either during first-run setup or in **Settings**.
2. Open the pack library and choose **Install** on the modpack you want to update. That sets the destination for you. (A pack outside the CurseForge library can be pointed at by hand instead.)
3. Enter the update code provided by the admin (`modpack-key/uuid`, or just the UUID for older updates).
4. Preview exactly what will be added, updated and deleted before anything is written.
5. Install. Nothing touches the pack until you confirm, and an interrupted install rolls back rather than leaving it half-written.

### Settings
- GitHub repository the update code resolves against — the one an admin publishes to and a user downloads from.
- GitHub token, used only to publish. Downloads are unauthenticated, so the repository has to be public for users to install from it; the token is not a way to distribute from a private repo. Stored in the OS keyring.

## Getting Started (Contribute/Development)

### Prerequisites
- Bun Package Manager (>= 1.3.11) — this project uses Bun instead of Node/npm; `bun install` refuses to run under npm/yarn/pnpm
- Rust (>= 1.88.0)
- Git
- VS Code (recommended for development)

### Installation
1. Clone the repository:
    ```bash
    git clone https://github.com/YassaaaTU/cemm.git
    cd cemm
    ```
2. Install frontend dependencies using Bun:
    ```bash
    bun install
    ```

### Running the Project
To start the development app, run:
```bash
bun app
```

To build for production:
```bash
bun app:build
```

### Project Structure
```
cemm/
├── app/                # Nuxt 4 frontend
│   ├── components/     # Vue components
│   ├── pages/          # Application pages
│   ├── stores/         # Pinia state management
│   ├── composables/    # Reusable logic
│   └── types/          # TypeScript type definitions
├── src-tauri/          # Tauri backend
│   ├── src/            # Native shell, sidecar client/service, and Rust domains
│   └── Cargo.toml      # Rust dependencies
├── docs/architecture/  # Accepted architecture decisions
└── .plan/              # Implementation plans for completed rewrites
```

### Runtime Architecture

The Tauri host starts the packaged CEMM executable in a private local-service
mode and communicates with it over inherited stdin/stdout. Tauri owns native OS
integration; the sidecar owns filesystem, manifest, GitHub, CurseForge library,
and installation work. No local HTTP port or separately installed daemon is
used. See [ADR-001](docs/architecture/ADR-001-local-rust-sidecar.md) for the
decision, boundaries, and failure behavior.

### Tech Stack
- **Frontend**: Nuxt 4, Vue 3, TypeScript, Tailwind CSS v4, DaisyUI, Pinia
- **Native shell**: Tauri (dialogs, window integration, keyring, updater)
- **Local service**: Rust, Serde, Tokio, newline-delimited JSON over inherited stdio
- **Storage**: GitHub API, Tauri Keyring (secure token storage)
- **Development**: Bun package manager, ESLint, Pino logging
- **Build**: Tauri bundler for cross-platform desktop apps
