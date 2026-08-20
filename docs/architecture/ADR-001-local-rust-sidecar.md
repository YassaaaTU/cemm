# ADR-001: Run privileged application work in a local Rust sidecar

**Status:** Accepted
**Date:** 2026-08-20
**Deciders:** CEMM maintainers

## Context

CEMM is a packaged desktop application. Its Nuxt webview needs native dialogs,
window control, keyring access, and updater integration, while its domain work
reads and writes modpack files, parses manifests, scans CurseForge instances,
publishes updates, and installs downloaded content.

Previously, all native and domain work ran in the Tauri host process. That made
the host both the UI shell and the privileged application backend. A domain
failure could therefore affect the shell, domain operations had no independent
lifecycle, and concurrent install or publish requests had no single execution
boundary.

The rewrite must preserve existing frontend command names and serialized update
formats. It must not require hosted infrastructure, a separately installed
daemon, an exposed local port, or a database. The application must remain a
single distributable executable.

## Decision

The packaged CEMM executable supports a private `--cemm-sidecar-service` mode.
On startup, the Tauri host launches the same executable as a child process and
communicates through newline-delimited JSON on inherited stdin and stdout.

The protocol is versioned and contains request IDs plus distinct ready,
response, error, and event messages. The host waits for a ready handshake before
showing a usable application. It forwards sidecar progress events to the
webview and fails pending requests explicitly if the child exits or sends an
invalid protocol message. The child handles one request at a time, deliberately
serializing privileged operations.

Ownership is divided as follows:

| Owner | Responsibilities |
|---|---|
| Nuxt webview | Presentation, preview state, user intent, progress display |
| Tauri host | Sidecar lifecycle, native dialogs, window/open-URL actions, keyring, updater, restart, event forwarding |
| Rust sidecar | Filesystem/import operations, path validation, manifest work, GitHub distribution, CurseForge library/cache, installation |

Existing Tauri command names remain compatibility adapters. Domain commands do
not execute domain logic in the host; they translate the current payload into a
typed sidecar call and return its result or error.

## Options Considered

### Option 1: Keep all work in the Tauri host

| Dimension | Assessment |
|---|---|
| Complexity | Low initially |
| Packaging cost | None |
| Failure isolation | Low |
| Operational overhead | Low |

**Pros:** No new protocol or child lifecycle.

**Cons:** Retains mixed responsibilities, weak failure isolation, and no central
serialization boundary for installs and publishes.

### Option 2: Use a loopback HTTP service

| Dimension | Assessment |
|---|---|
| Complexity | Medium to high |
| Packaging cost | Additional server/runtime concerns |
| Failure isolation | High |
| Local attack surface | Higher than inherited pipes |

**Pros:** Familiar tooling and independently addressable endpoints.

**Cons:** Requires port selection, authentication, discovery, shutdown handling,
and protection from other local processes. HTTP adds no product value for a
single parent and child.

### Option 3: Use an inherited-stdio Rust sidecar

| Dimension | Assessment |
|---|---|
| Complexity | Medium |
| Packaging cost | No second artifact |
| Failure isolation | High |
| Local attack surface | Narrow |

**Pros:** Real process isolation, private transport, no port or daemon, one
packaged executable, and deterministic request ordering.

**Cons:** Introduces a protocol compatibility surface and requires explicit
startup, shutdown, event forwarding, and child-failure handling.

## Trade-off Analysis

Option 3 adds more lifecycle code than the in-process design, but it creates the
requested service boundary without the operational and security costs of an
HTTP listener. Reusing the current executable avoids platform-specific sidecar
artifact naming and signing. Sequential dispatch is intentional: CEMM does not
need parallel privileged mutations, and serialization prevents two installs or
publishes from racing over shared local state.

This is a service-oriented architecture, not a microservice deployment. Splitting
each domain into another process, adding a queue, or hosting an API would add
failure modes without a measured need.

## Failure and recovery behavior

- Startup fails if the child cannot launch, misses the five-second handshake, or
  reports a different protocol version.
- Each domain failure is returned as a service error associated with its request
  ID; publish and install callers preserve these as thrown failures.
- Unexpected child exit or malformed output closes the client and rejects every
  pending request instead of leaving the UI waiting indefinitely.
- Installation validates all add-on URLs, destinations, config paths, and binary
  payloads before cleanup. Downloads stage before old add-ons are removed.
- The sidecar records `cemm-manifest.json` only after installed files are written.
  Full filesystem rollback is not claimed; errors after promotion report the
  partial completion explicitly.
- The Tauri host owns the child handle, closes its stdin, terminates the child,
  and waits for it when the application shuts down.

## Consequences

- Domain crashes and protocol failures are isolated from the webview process.
- Tauri remains a thin native shell instead of a second domain implementation.
- Privileged mutations have one serialized owner.
- Protocol version 1 and existing Tauri command payloads are compatibility
  surfaces that require contract tests when changed.
- Progress crosses the sidecar and Tauri boundaries before reaching the UI.
- The current executable can be started manually in service mode for packaged
  smoke tests without opening a native window.

## Action Items

1. [x] Add the versioned stdio protocol and managed child lifecycle.
2. [x] Migrate filesystem, manifest, GitHub, CurseForge library, and install work.
3. [x] Keep only native-shell responsibilities in direct Tauri handlers.
4. [x] Centralize frontend IPC access in `useTauri`.
5. [x] Add protocol, dispatch-contract, domain safety, and temporary-install tests.
6. [x] Keep lint, typecheck, generation, rustfmt, clippy, and Rust tests in CI.
7. [ ] Validate a native Tauri window and live GitHub publish manually before release.
