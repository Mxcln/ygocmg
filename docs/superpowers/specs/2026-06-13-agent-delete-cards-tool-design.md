# Agent Delete Cards Tool Design

## Context

YGOCMG's AI Agent can currently create, update, and move cards, but it cannot delete cards through a tool call. The application already has backend and frontend API support for card deletion:

- `cardApi.deleteCard` wraps the single-card `delete_card` command.
- `cardApi.bulkDeleteCards` wraps the batch `bulk_delete_cards` command.
- `cardApi.confirmCardBatchWrite` confirms batch card writes.
- The existing `move_cards` agent tool is already batch-shaped: it accepts `cardIds: string[]` and calls `cardApi.moveCards`. A single-card move is represented as an array with one id.

The new agent deletion capability should follow the same batch-shaped pattern as `move_cards`.

## Decision

Add one agent tool named `delete_cards`.

`delete_cards` supports both single-card and multi-card deletion by accepting a non-empty `cardIds` array. It will call `cardApi.bulkDeleteCards` rather than exposing both single-card and batch deletion tools. This keeps the model-facing tool surface small and aligns deletion with the current move tool.

## Tool Contract

Tool name: `delete_cards`

Parameters:

- `cardIds: string[]` required. Card ids come from `list_cards` or from the current selected/checked card context injected into the agent state.
- `deleteAssets?: boolean` optional. Defaults to `true`, matching the current product behavior that card deletion removes associated main images, field images, and scripts.

Behavior:

- Requires an active workspace and active custom pack via the existing `requirePack` helper.
- Validates that `cardIds` is a non-empty array before calling the API.
- Calls `cardApi.bulkDeleteCards({ workspaceId, packId, cardIds, deleteAssets })`.
- Uses `cardApi.confirmCardBatchWrite` for `needs_confirmation` results.
- Invalidates the default card caches after successful write or confirmed write.

## Agent Guidance

Update the system prompt so the model knows:

- `delete_cards` is destructive and may ask the user for confirmation in the chat UI.
- It should use `list_cards` or selected/checked card context to identify ids before deleting.
- It can delete one card by passing a one-element `cardIds` array.
- Deletion defaults to removing associated assets.
- `move_cards` already has the same batch shape and also accepts one or more card ids.

## Code Changes

Frontend:

- `src/features/agent/tools/writeTools.ts`
  - Add `deleteCardsTool`.
  - Import or reuse the existing `cardApi` and `WriteResult` types.
  - Add `confirmWrite: (confirmationToken) => cardApi.confirmCardBatchWrite({ confirmationToken })`.
  - Keep default cache invalidation by omitting `invalidateKeys`.
- `src/features/agent/tools/registry.ts`
  - Register `deleteCardsTool` with the other card write tools.
- `src/features/agent/systemPrompt.ts`
  - Mention `delete_cards` alongside `create_card`, `update_card`, and `move_cards`.

Docs:

- `docs/agent.md`
  - Update tool counts and card write tool list.
  - Describe `delete_cards` as the batch-shaped deletion tool.
- `docs/functional_spec.md`
  - Update the AI Agent section to include card deletion.

## Tests

Use TDD for implementation.

Add focused Vitest coverage for the tool layer:

- `delete_cards` rejects an empty or missing `cardIds` array without calling the API.
- `delete_cards` calls `cardApi.bulkDeleteCards` with the active workspace/pack, stringified ids, and `deleteAssets: true` by default.
- `delete_cards` passes `deleteAssets: false` when explicitly requested.
- The registered tool exposes `confirmWrite` that calls `cardApi.confirmCardBatchWrite`.

Run:

- `npm run test`
- `npm run typecheck`

## Out Of Scope

- No new backend command.
- No raw Tauri invoke from the agent.
- No separate `delete_card` agent tool.
- No workspace switching, standard pack mutation, or settings changes.
