# YGOCMG Architecture Map

This note is a compact guide to the current code layout and data flow. It is not a full design document.

## Runtime Entry Points

- `src/main.tsx` boots the frontend app.
- `src/app/App.tsx` owns application bootstrap, shell state restoration, and top-level routing between workspace, pack, and standard pack views.
- `src-tauri/src/main.rs` creates the Tauri app, manages shared state, and registers commands.
- `src-tauri/src/tauri_commands.rs` is the backend command surface exposed to the frontend.

## Frontend Boundaries

- `src/app` handles shell-level concerns such as window behavior, workspace bootstrap, modal orchestration, and top-level view switching.
- `src/features/*` contains domain-oriented UI modules such as workspace, pack, card, strings, export, import, settings, and standard pack.
- `src/shared/api` contains the frontend wrappers around Tauri commands.
- `src/shared/contracts` contains the typed request/response shapes shared across frontend and backend.
- `src/shared/stores` holds global UI/session state used across features.

## Backend Boundaries

- `domain` is the place for business entities, invariants, and rules.
- `application` is the place for use cases, orchestration, and cross-domain workflows.
- `infrastructure` is the place for filesystem, SQLite, asset, and external-system adapters.
- `runtime` is the place for session, cache, job, and event state that lives while the app is running.
- `presentation` and `tauri_commands` expose backend behavior to the frontend without leaking implementation details.

## Data Flow

Typical flow:

`React UI -> src/shared/api -> Tauri command -> application service -> domain/infrastructure -> contract DTO -> UI`

Practical implications:

- UI code should not reach around the API wrapper layer for backend work.
- Shared contracts should describe the public shape of the data, not the internal storage model.
- If a workflow crosses persistence, filesystem, or card-resource boundaries, the backend should own the rule.

