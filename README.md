# CEMM - ChillEcke Modpack Manager

## Overview
CEMM (ChillEcke Modpack Manager) is a lightweight desktop application that makes it easier for you and your friends to play pre-existing CurseForge modpacks with custom modifications. Built with Nuxt 3 frontend and Tauri/Rust backend, it provides two main modes:

- **Admin Mode**: Modify a downloaded CurseForge modpack (add, remove, or update addons and config files) → Generate UUID and upload changes to GitHub
- **User Mode**: Paste the UUID code from admin → Select correct modpack directory → Install the modifications automatically

## Features
- **Modpack Modification**: Add, remove, or update mods/resourcepacks/shaderpacks from CurseForge modpacks
- **Config File Distribution**: Share custom configuration files with your friends
- **Easy Sharing**: Generate UUID codes for simple modpack distribution
- **GitHub Integration**: Secure distribution via GitHub repositories
- **Automatic Installation**: Users can install modifications with a single UUID code
- **Cross-Platform**: Works on Windows *AND SHOULD* work on macOS and Linux, never tested on these platforms

## Getting Started

### Prerequisites
- Node.js (>= 18.x)
- Bun Package Manager
- Rust (>= 1.70)
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

## Usage

### Admin Mode
1. Select your modpack directory containing `minecraftinstance.json`
2. Select the minecraftinstance.json file to generate `manifest.json`
   - This file contains metadata about your modpack
   - It will be used to track updates and changes
3. Choose config files to distribute (optional)
4. Generate UUID and upload to your GitHub repository
5. Share the UUID with users for easy installation

### User Mode
1. Configure your GitHub repository settings
2. Enter the UUID code provided by the admin
3. Preview what changes will be applied
4. Install the update to your modpack directory

### Settings
- GitHub repository name from which to download updates using the UUID
- Github token for secure access to private repositories (only needed for admin mode)

## Development

### Project Structure
```
cemm/
├── app/                    # Nuxt 3 frontend
│   ├── components/         # Vue components
│   ├── pages/             # Application pages
│   ├── stores/            # Pinia state management
│   └── composables/       # Reusable logic
├── src-tauri/             # Tauri backend
│   ├── src/               # Rust source code
│   └── Cargo.toml         # Rust dependencies
└── types/                 # TypeScript type definitions
```

### Tech Stack
- **Frontend**: Nuxt 3, Vue 3, TypeScript, Tailwind CSS v4, DaisyUI, Pinia
- **Backend**: Tauri, Rust, Serde, Tokio
- **Storage**: GitHub API, Tauri Keyring (secure token storage)
- **Development**: Bun package manager, ESLint, Pino logging
- **Build**: Tauri bundler for cross-platform desktop apps
