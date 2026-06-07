# Card Domain Glossary

This glossary keeps the core domain terms stable so future work does not need to rediscover them from scratch.

## Core Terms

- Workspace: a user-level working context that groups packs, config, and recent session state.
- Custom Pack: a user-editable pack owned by the workspace.
- Standard Pack: read-only reference data used for inspection, lookup, and comparison.
- CardEntity: the main card data model used by the application.
- CardTexts: localized text fields attached to a card.
- Pack Strings: named string records associated with a pack.
- Text Language Catalog: the available language keys and display labels used by card and pack text flows.
- Preview Token / Confirmation Token: temporary tokens used to confirm a write after a preview or validation step.
- Job: a background operation that may outlive a single UI interaction.

## Important Distinctions

- Standard pack data is reference data and should not be edited as user-owned content.
- Custom pack data is the editable source of truth for the user's workspace.
- A `setname` can depend on language and is used as a high-level lookup/indexing concept, not as a free-form label.
- The current functional spec is the source of truth for behavior when this glossary and older design docs differ.

## Resource Concepts

- Main card image: the primary card art resource.
- Field image: a resource associated with the field/background context.
- Script: the card script resource used by the YGOPro-compatible runtime.
- YGOPro path: the local resource root used when the app resolves external card assets.
- Code changes can affect resource consistency, especially when a card code changes and related assets need to move with it.

