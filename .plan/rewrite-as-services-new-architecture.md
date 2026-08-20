# Local Rust Sidecar Rewrite Plan

**Status:** Implemented
**Date:** 2026-08-20
**Branch:** `feature/rewrite-as-services-new-architecture`

## Context

CEMM currently runs privileged filesystem, network, manifest, library-scan, and
installation work directly in the Tauri host process. The rewrite must create a
real process boundary without replacing the Nuxt UI, changing update codes, or
requiring hosted infrastructure. Every migration slice must leave the desktop
application runnable, preserve the current serialized contracts, and avoid
touching real modpack instances during verification.

## Decision

Run the local service as a child process of the packaged CEMM executable. The
child is selected by a private `--cemm-sidecar-service` argument and communicates
with the Tauri host through newline-delimited JSON over inherited stdin/stdout.

- No loopback port, HTTP server, external daemon installer, or service discovery.
- The Tauri host owns sidecar startup, shutdown, and event forwarding.
- The sidecar serializes privileged operations, which also prevents concurrent
  installs or publishes from racing.
- Tauri keeps window, dialog, keyring, updater, and process-restart integration.
- The sidecar owns filesystem operations, manifest work, GitHub distribution,
  CurseForge library scanning/cache, and installation.
- Existing frontend command names and TypeScript payloads remain stable while
  their Rust handlers become thin service proxies.

## Trust Boundaries

1. Vue webview -> Tauri commands: presentation and user intent only.
2. Tauri host -> sidecar stdio: private child-process channel with typed request
   IDs, results, errors, and progress events.
3. Sidecar -> local filesystem: all path and install validation remains in Rust.
4. Sidecar -> GitHub/CurseForge: existing HTTPS allowlists, limits, and repository
   validation remain binding.
5. Tauri updater -> signed GitHub release artifacts: unchanged.

## Migration Tickets

### Ticket 1 - Service foundation

- Add request/response/event protocol types.
- Add the stdio service runner and Tauri-side process client.
- Start and health-check the child during Tauri setup.
- Add protocol, lifecycle, malformed-request, and unknown-method tests.
- Keep all existing feature commands on their current implementation.

### Ticket 2 - Filesystem and manifest service

- Route read/write/import/path validation through the sidecar.
- Route instance parsing and manifest comparison through the sidecar.
- Preserve dialog commands in Tauri.
- Preserve existing command names and frontend payloads.

### Ticket 3 - GitHub distribution service

- Move upload/download dispatch behind the sidecar.
- Forward `upload_progress` events through the Tauri host.
- Keep OS-keyring access in Tauri/frontend and pass the token only through the
  inherited pipe for the duration of a publish request.

### Ticket 4 - CurseForge library service

- Pass the Tauri-resolved cache directory into the child at startup.
- Route library scans and icon caching through the sidecar.
- Keep scanning offline-first and artwork fetching asynchronous from the UI.

### Ticket 5 - Installation service and commit boundary

- Route installs and `install-progress` events through the sidecar.
- Make the sidecar own staging, validation, addon/config promotion, and the final
  installed-manifest record as one recoverable operation.
- Keep the update preview visible and never run a real install for verification.

### Ticket 6 - Cutover and hardening

- Remove superseded direct implementations and direct frontend `invoke` calls.
- Replace ambiguous sentinel failures with explicit service errors where the UI
  can distinguish cancellation, absence, and failure.
- Add focused sidecar dispatch-contract tests for the existing frontend publish
  and install payloads without introducing a second frontend test framework.
- Update architecture documentation and CI checks.
- Run lint, typecheck, Rust formatting/clippy/tests, static generation, and a
  packaged sidecar smoke test.

## Consequences

- Privileged work can fail without taking down the webview/Tauri shell.
- The process protocol becomes a compatibility surface and must remain tested.
- Progress must cross two boundaries instead of one.
- A single serialized service is intentionally sufficient; separate daemons,
  queues, databases, or hosted APIs require measured need and a new ADR.
