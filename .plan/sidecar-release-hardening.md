# Sidecar Release Hardening Plan

**Status:** In progress
**Target:** CEMM 2.0.0
**Branch:** `master`
**Date:** 2026-08-20

## Objective

Close the release-readiness gaps found after the option-3 architecture merge
without changing its topology: one packaged CEMM executable, a Tauri shell, and
a private Rust sidecar over inherited stdin/stdout.

## Slice 1 - Recoverable installation finalization

- Stage add-ons, configs, and the installed manifest before mutating live files.
- Journal every live path that the operation can replace or remove.
- Move existing files into a transaction backup before promotion.
- Roll back on any cleanup, promotion, config, or manifest failure.
- Recover an interrupted transaction on the next install attempt.
- Validate old-manifest destinations before using them as cleanup targets.
- Add failure-injection tests for rollback and interrupted recovery.

Acceptance: a failure after cleanup begins restores the original instance, new
files are removed, and no new installed manifest is recorded.

## Slice 2 - Sidecar lifecycle and request resilience

- Give the client a supervisor that can replace a stopped child before the next
  request without retrying an ambiguous in-flight mutation.
- Add method-appropriate overall request deadlines.
- Stop and reap the child when a deadline expires.
- Make managed-state teardown deterministically close stdin, terminate, and
  wait for the child even while the reader thread exists.
- Add focused lifecycle tests.

Acceptance: pending calls fail explicitly, a subsequent call can restart a dead
service, timed-out children are reaped, and application teardown does not leave
the service running.

## Slice 3 - Correlated progress events

- Generate an operation ID in each publish/install frontend action.
- Carry it through the Tauri proxy and sidecar request.
- Include it in every progress event.
- Ignore progress events that do not match the active operation.
- Extend dispatch contract tests.

Acceptance: simultaneous listeners cannot consume another operation's progress.

## Slice 4 - Packaging proof and release gates

- Add an integration test that starts the actual compiled CEMM binary in
  sidecar mode, verifies ready/ping/error messages, closes stdin, and proves a
  clean child exit.
- Run backend CI on Windows, macOS, and Linux so the self-spawn topology is
  exercised on every supported packaging platform.
- Add a CEMM 2.0 manual release checklist for signed artifacts, native window
  startup, authenticated GitHub publish/download, updater metadata, and a
  disposable modpack install/recovery test.
- Update ADR consequences and completed action items honestly.

Acceptance: automated real-process smoke tests pass cross-platform; live
credentials, signing, updater, and native UX remain explicit manual release
gates rather than implied coverage.

## Verification after every slice

- `cargo fmt -- --check`
- focused Rust tests for the changed boundary
- `cargo check --all-targets`
- `bun run typecheck` for any frontend contract change
- `graphify update .` after the final code slice

Final gate: lint, typecheck, Nuxt generation, rustfmt, clippy with warnings
denied, all Rust tests, production Tauri build, optimized sidecar smoke test,
clean working tree.
