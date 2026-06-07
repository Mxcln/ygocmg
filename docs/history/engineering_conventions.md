# Engineering Conventions

This document captures the project habits that matter most when making changes.

## Frontend Conventions

- Put feature code in `src/features/<area>` when the change belongs to one domain area.
- Keep reusable API wrappers, contracts, stores, utilities, and i18n helpers in `src/shared`.
- Use `src/shared/api` as the normal way to talk to Tauri commands.
- Prefer the existing React Query patterns for async data that is fetched, cached, or refreshed.
- Prefer the existing Zustand shell store for cross-feature shell state.
- Keep UI changes consistent with the current CSS module, theme, and i18n approach instead of introducing a new UI framework for a small change.

## Backend Conventions

- Keep business rules in the backend rather than duplicating them in the frontend.
- Use `application` for orchestration and use cases.
- Use `infrastructure` for filesystem, SQLite, asset, and other adapter work.
- Keep `tauri_commands` thin and focused on boundary adaptation.
- When a workflow crosses persistence or file-system boundaries, let the backend own the rule and validation.

## Documentation Conventions

- The current functional spec is the first place to check for current behavior.
- Historical design docs are background material unless the current spec still agrees with them.
- Add or update an `docs/ai/*.md` note when a change creates a stable fact about architecture, domain vocabulary, module boundaries, or verification flow.
- Do not copy long sections of the existing product specs into the AI knowledge layer.

