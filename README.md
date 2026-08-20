# CEMM - ChillEcke Modpack Manager

CEMM (ChillEcke Modpack Manager) is a lightweight desktop application that makes it easier for you and your friends to play pre-existing CurseForge modpacks with custom modifications. Built with a Nuxt 4 frontend, a Tauri native shell, and a local Rust sidecar service, it provides two main modes:

- **Admin Mode**: Modify a downloaded CurseForge modpack (add, remove, or update addons and config files) → Generate an update code and upload changes to GitHub
- **User Mode**: Paste the update code from admin → Select correct modpack directory → Install the modifications automatically

## Features
- **Modpack Modification**: Add, remove, or update mods/resourcepacks/shaderpacks/datapacks from CurseForge modpacks
- **Addon Exclusion**: Exclude specific mods or resourcepacks from the uploaded instance without removing them locally
- **Config File Distribution**: Share custom configuration files with your friends
- **Easy Sharing**: Generate update codes for simple modpack distribution
- **GitHub Integration**: Secure distribution via GitHub repositories
- **Automatic Installation**: Users can install modifications with a single update code
- **Cross-Platform**: Works on Windows *AND SHOULD* work on macOS and Linux, never tested on these platforms

## Usage

### Admin Mode
1. Select your modpack directory containing `minecraftinstance.json`
2. Select the minecraftinstance.json file to generate `manifest.json`
   - This file contains metadata about your modpack
   - It will be used to track updates and changes
3. Choose config files to distribute (optional)
4. **Exclude addons (optional)**: Click the ban icon (🚫) on any mod or resourcepack to exclude it from the upload
   - Excluded addons are marked with strikethrough styling
   - They remain in your local modpack but won't be included in the distributed instance
   - Useful for keeping server-side mods private or excluding platform-specific addons
5. Generate an update code and upload to your GitHub repository
6. Share the update code with users for easy installation

### User Mode
1. Configure your GitHub repository settings
2. Enter the update code provided by the admin (`modpack-key/uuid`, or just the UUID for older updates)
3. Preview what changes will be applied
4. Install the update to your modpack directory

### Settings
- GitHub repository name from which to download updates using the update code
- Github token for secure access to private repositories (only needed for admin mode)

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
└── .plan/              # Implementation plans for active rewrites
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
