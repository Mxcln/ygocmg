# Testing and Verification

This document collects the checks that are useful when making changes in this repository.

## Common Checks

- Frontend typecheck: `npm run typecheck`
- Full frontend build: `npm run build`
- Tauri/Rust checks: run `cargo check` or `cargo test` from `src-tauri` when the backend changes touch Rust logic

## Verification Strategy

- Prefer `npm run typecheck` first for TypeScript and contract-level changes.
- Run `npm run build` when the change may affect bundling, module resolution, or app startup.
- For backend changes, use a Rust check or test command from `src-tauri` so the backend path is exercised directly.

## Current Script Surface

- `package.json` currently defines `dev`, `build`, `typecheck`, and `tauri`.
- There is no separate `test` or `lint` npm script defined right now, so do not assume one exists unless the repo adds it later.

## Reporting Failures

When a check fails, record:

- what command ran
- what failed
- whether the failure looks like a new regression or an existing repo issue
- whether the next step is a code fix, a config fix, or a follow-up verification run

