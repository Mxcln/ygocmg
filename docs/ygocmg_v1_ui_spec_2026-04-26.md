# YGOCMG v1 UI Specification

Date: 2026-04-26  
Status: Draft

Related documents:
- [Implementation Packages](./implementation_packages.md)
- [YGOCMG v1 Functional Specification](./ygocmg_v1_functional_spec_2026-04-25.md)
- [YGOCMG v1 Interface Design](./ygocmg_v1_interface_design_2026-04-25.md)
- Reference sketch: `../pics/overall.jpg`

## 1. Purpose

This document defines the authoritative v1 desktop UI specification for the main YGOCMG authoring shell.

It is intended to be precise enough that another engineer can implement the UI without making product decisions about:

1. Main application layout
2. Navigation model
3. Window chrome behavior
4. Pack-focused editing flow
5. Modal and drawer behavior
6. Read-only vs editable states

This document complements the functional specification and interface design documents:

1. The functional specification defines what the product must support
2. The interface design document defines long-term backend and DTO structure
3. This UI document defines what the user sees and how the shell behaves

## 2. Global UI Principles

### 2.1 Platform target

v1 is Windows-first.

This affects:

1. Custom titlebar control placement
2. Window drag behavior assumptions
3. Maximize / minimize / close expectations
4. Visual proportions for the titlebar area

macOS and Linux adaptations are intentionally out of scope for this document.

### 2.2 Language policy

UI is English-first in v1.

This applies to:

1. Visible UI text
2. Component names
3. View names
4. Tab names
5. Modal names
6. State names in examples

Chinese localization is deferred to a future i18n layer and must not shape the v1 shell layout.

### 2.3 Screen usage

The baseline target window is `1280x800`, matching the current Tauri desktop configuration.

The UI must be optimized for one-screen usage:

1. No full-page vertical scroll in the main shell at `1280x800`
2. No full-page horizontal scroll
3. Scroll is allowed only inside local content regions
4. Large onboarding panels and dashboard hero sections are forbidden

### 2.4 Navigation model

The main authoring shell is pack-centric, not page-centric.

The shell must not use:

1. A dashboard landing page
2. Persistent top-level route switching for workspace/settings/export
3. Separate pages for card editing

The shell must use:

1. A single main desktop window
2. Centered modals for utility flows
3. A persistent left sidebar for pack navigation
4. A right-side overlay drawer for card editing

## 3. Main Window Shell

The main window is composed of five coordinated layers:

1. Custom titlebar
2. Left sidebar
3. Right work area
4. Centered modal layer
5. Right overlay drawer layer

### 3.1 Layout overview

```text
+----------------------------------------------------------------------------------+
| Titlebar                                                                         |
| [App Icon] [Current Workspace Name]                  [min] [max] [close]        |
+--------------------------+-------------------------------------------------------+
| Sidebar                  | Work Area                                             |
|                          |                                                       |
| [Workspace]              | [Pack Metadata Summary Bar]                           |
| [Export Expansions]      |                                                       |
| [Global Settings]        | [Cards | Strings]                                     |
|                          |                                                       |
| [Opened Pack A]          | [Current Pack Content]                                |
| [Opened Pack B]          |                                                       |
| [+]                      |                                                       |
| ...                      |                                                       |
| [Standard Pack]          |                                                       |
+--------------------------+-------------------------------------------------------+
| Optional overlay: Card Editor Drawer covers the full right work area             |
+----------------------------------------------------------------------------------+
```

### 3.2 Shell states

The main shell must support these states:

1. No workspace open
2. Workspace open, no custom pack open
3. Workspace open, one custom pack active
4. Workspace open, multiple custom packs open
5. Standard Pack active
6. Card editor drawer open
7. Utility modal open

### 3.3 Session restoration

On application startup:

1. The app automatically reopens the most recently used workspace (determined by `last_opened_at` in the workspace registry)
2. After the workspace is restored, all previously opened pack tabs are reopened using `open_pack_ids` persisted in the workspace metadata
3. The previously active pack is restored using `last_opened_pack_id`
4. If no workspace has been opened before, the shell shows the empty state
5. If a previously opened workspace or pack no longer exists on disk, the restoration silently skips it

### 3.4 Default main-pane behavior

If a workspace is open and no custom pack is currently open:

1. The right work area shows a compact empty state
2. The empty state prompts the user to open or create a pack
3. Standard Pack is not opened automatically

If the pinned Standard Pack is clicked:

1. The right work area switches into Standard Pack view
2. That view uses the same top-level layout as a normal pack
3. All actions inside that view are read-only

## 4. Custom Titlebar

## 4.1 Purpose

The titlebar replaces the native framed titlebar and provides:

1. Application identity
2. Current workspace context
3. Native window controls
4. Window drag region

## 4.2 Layout

From left to right:

1. App icon
2. Current workspace name
3. Flexible drag region
4. Minimize button
5. Maximize / restore button
6. Close button

### 4.3 Visible text

The workspace text must display:

1. The current workspace name when a workspace is open
2. `No Workspace Open` when no workspace is open

The titlebar must not include:

1. A page title
2. Breadcrumbs
3. Help text
4. Welcome copy

### 4.4 Drag behavior

The titlebar background is draggable except for interactive controls.

Non-draggable regions:

1. App icon button if clickable
2. Window control buttons
3. Any future explicit interactive titlebar control

### 4.5 Window control behavior

Buttons are Windows-style and right-aligned.

Required behavior:

1. Minimize sends the app window to the taskbar
2. Maximize toggles to maximized
3. Restore toggles from maximized to normal size
4. Close requests app shutdown

### 4.6 Titlebar states

The specification must support these visual states:

1. No workspace open
   - Workspace text shows `No Workspace Open`
2. Workspace open
   - Workspace text shows the current workspace name
3. Maximized window
   - Maximize button switches to restore icon/state
4. Inactive window
   - Titlebar colors and control emphasis become muted

### 4.7 API dependency status

| Capability | Status | Notes |
|---|---|---|
| Read current workspace name | Available now | Current workspace metadata already exists in runtime/session state |
| Minimize/maximize/close window actions | Available now | Implemented via `@tauri-apps/api/window` with Tauri v2 capability permissions (`core:window:allow-minimize`, `allow-toggle-maximize`, `allow-close`) |
| Detect maximized state | Available now | Implemented via `appWindow.isMaximized()` with `core:window:allow-is-maximized` permission |
| Detect active/inactive window state | Available now | Implemented via `appWindow.onFocusChanged()` with `core:window:allow-is-focused` permission |
| Window drag region | Available now | Implemented via `data-tauri-drag-region` with `core:window:allow-start-dragging` permission |

## 5. Left Sidebar

## 5.1 Purpose

The left sidebar provides:

1. Utility actions
2. Opened pack navigation
3. Entry point for pack creation/open/import
4. Persistent access to Standard Pack

The sidebar is always visible, even when the card editor drawer is open.

### 5.2 Structure

The sidebar has two sections:

1. Top action strip
2. Pack stack

## 5.3 Top action strip

The top action strip contains exactly three icon buttons, in this order:

1. `Workspace`
2. `Export Expansions`
3. `Global Settings`

Rules:

1. Each button opens a centered modal
2. None of these buttons switches the main page
3. None opens a separate app window
4. None opens an anchored popover

### 5.4 Pack stack rules

The pack stack below the action strip must contain:

1. All currently opened custom packs, in runtime open order
2. A `+` tile
3. A pinned `Standard Pack` tile at the bottom

Additional rules:

1. The left sidebar is the only pack navigation in the shell
2. There is no second pack tab strip in the main pane
3. Unopened custom packs do not appear in the stack
4. Unopened custom packs are accessed through the `Open Pack` tab in the `+` modal

### 5.5 Pack tile visual states

Each sidebar item must have one of these visual states:

1. Active custom pack
2. Inactive opened custom pack
3. `+` action tile
4. Pinned Standard Pack

Required distinction:

1. Active custom pack
   - Highest emphasis
   - Clear selected background/border treatment
2. Inactive custom pack
   - Neutral treatment
3. `+` tile
   - Explicit action appearance
   - Not visually confused with a pack
4. Standard Pack
   - Pinned
   - Read-only appearance
   - Visually distinct from editable custom packs

### 5.6 Sidebar overflow

The sidebar itself must not trigger full-window scrolling.

If many packs are opened:

1. The pack stack becomes a local scroll region
2. The action strip remains visible
3. The pinned Standard Pack remains visible at the bottom if possible
4. If the full pinned behavior is not technically feasible in the first implementation, Standard Pack must remain the last item in the stack and preserve visual identity

### 5.7 Sidebar interactions

Custom pack tile:

1. Single-click switches active pack
2. It does not open a modal
3. Close behavior, if present, must be an explicit close affordance and not implicit on selection

`+` tile:

1. Opens a centered modal
2. Default tab is `Open Pack`

Standard Pack tile:

1. Single-click opens Standard Pack in the right work area
2. Standard Pack is never editable

### 5.8 API dependency status

| Capability | Status | Notes |
|---|---|---|
| Track currently opened custom packs | Available now | Runtime session already maintains `open_pack_ids` |
| Track active pack | Available now | Runtime session already maintains `active_pack_id` |
| Open a pack | Available now | `open_pack` command exists |
| Close a pack | Partially available | Backend close behavior exists in service design, but current Tauri command surface does not expose a close-pack command |
| List all custom packs in workspace for `Open Pack` tab | Partially available | `list_pack_overviews` exists, but current frontend shell does not yet consume it for the modal flow |
| Standard Pack read-only state | Future API required | No current Standard Pack frontend/backend command surface is exposed |

## 6. Right Work Area

## 6.1 Purpose

The right work area is the primary content surface for:

1. Current pack summary
2. Pack content tabs
3. Standard Pack read-only browsing
4. Empty state when no custom pack is open

### 6.2 Top-level vertical structure

The right work area contains:

1. Pack metadata summary bar
2. Main tabbed content region

The content region below the summary bar is the only normal local scroll container area for pack content.

## 6.3 Empty state

When a workspace is open and no custom pack is active:

1. Show a compact empty state
2. The empty state must stay within the one-screen layout
3. It must include:
   - a short label such as `No Pack Open`
   - a prompt to use `+` to open or create a pack
4. It must not include:
   - a full onboarding page
   - large cards
   - marketing-style copy

## 6.4 Pack metadata summary bar

### Purpose

This bar exposes pack identity and editing context without forcing the user to open a full metadata form all the time.

### Collapsed state

Collapsed metadata must always show:

1. Pack name
2. Author
3. Version
4. Languages

The right edge contains a chevron control that expands/collapses the panel.

### Expanded state

Expanded metadata shows the full pack metadata panel.

For custom packs, it must expose:

1. Pack name
2. Author
3. Version
4. Description
5. Display language order
6. Default export language
7. Created at
8. Updated at
9. Card count summary

For Standard Pack:

1. The same surface is used
2. Fields are presented read-only
3. Standard Pack-specific status may replace custom-pack-only controls

### State rules

1. Collapse/expand is local to the current active pack view
2. Collapsed is the default state
3. Expanded content must not force full-window scrolling
4. Expanded content may use a local content body with constrained height if needed

### API dependency status

| Capability | Status | Notes |
|---|---|---|
| Pack name/author/version | Available now | Present in `PackMetadata` / `PackOverview` |
| Languages summary | Available now | Present in pack metadata |
| Full pack metadata for active custom pack | Available now | `open_pack` returns pack metadata |
| Standard Pack metadata/status | Future API required | Standard Pack read-only integration is not yet exposed |

## 6.5 Main tabbed content region

The main content region contains exactly two top-level tabs:

1. `Cards`
2. `Strings`

These tabs switch within the same pack view and must not change the outer shell layout.

### Shared rules

1. The tab strip stays visible while the content body scrolls locally
2. The tab body is the primary local scroll region
3. Tab switching does not reset the active pack
4. Tab switching does not close the card editor drawer automatically unless the selected card becomes invalid in context

## 6.6 `Cards` tab

### Purpose

The `Cards` tab is the primary card-list management surface for the active pack.

### Visible layout

The `Cards` tab must contain:

1. A compact toolbar area
2. A card list region

Recommended toolbar contents:

1. Search input
2. Sort control
3. New card action
4. Optional list density/filter controls if space allows

### Card list behavior

1. Single-clicking a card row opens the editor drawer
2. The card list itself remains visible beneath the drawer layer when the drawer is closed
3. The list is a local scroll region
4. The list must be usable without forcing shell-level scrolling

### Read-only rules

If the current active view is Standard Pack:

1. The list is browseable
2. Editing actions are hidden or disabled
3. Selecting a row does not open the editable card drawer

### API dependency status

| Capability | Status | Notes |
|---|---|---|
| List cards for active custom pack | Available now | `list_cards` exists |
| Create/update/delete cards | Partially available | Backend commands exist, but the drawer-level editable snapshot flow is not yet defined in the current frontend shell |
| Standard Pack card browsing | Future API required | Read-only standard pack browsing APIs are not exposed in the current app |

## 6.7 `Strings` tab

### Purpose

The `Strings` tab manages pack-level `strings.conf`-style entries for the active pack.

### Visible layout

The `Strings` tab must contain:

1. A compact toolbar area
2. A strings list/grid area

Recommended toolbar contents:

1. Language selector
2. Kind filter
3. Key filter or search field
4. Add string action

### Behavior

1. The tab content body is a local scroll region
2. Strings editing stays inside the right work area
3. Standard Pack strings view is read-only

### API dependency status

| Capability | Status | Notes |
|---|---|---|
| Strings tab shell | Future API required | No current frontend/backend strings command surface is exposed in the app shell |
| Standard Pack strings reference browsing | Future API required | Standard Pack read-only string APIs are not exposed |

## 7. Card Editor Drawer

## 7.1 Purpose

The card editor drawer is the primary editing surface for an individual card.

It is not a page and not a modal.

### 7.2 Open behavior

The drawer opens when the user single-clicks a card row in the `Cards` tab.

Rules:

1. The drawer overlays the entire right work area
2. The left sidebar remains visible
3. The active pack remains the same
4. Only one drawer is open at a time

### 7.3 Close behavior

The drawer must support:

1. Explicit close button
2. Escape key close when safe
3. Switching directly to another card row, which replaces drawer content

If there are unsaved edits:

1. Closing must require explicit user handling
2. Replacing the current card with another selected card must also require explicit handling

### 7.4 Layout

The drawer has two persistent zones:

1. Card preview zone
2. Editor tab zone

The card image must always remain visible in the drawer, regardless of the active internal tab.

### 7.5 Internal tabs

The drawer uses internal tabs, not a single long form.

Recommended v1 tabs:

1. `Basics`
2. `Texts`
3. `Assets`

`Assets` may include script-related controls if space and data model require it.

### 7.6 Tab contents

`Basics`:

1. Code
2. Type-related fields
3. Combat/stat fields
4. Core structural fields

`Texts`:

1. Language selection/editing context
2. Name
3. Description
4. Text string entries

`Assets`:

1. Main card image preview
2. Field image preview if relevant
3. Script state
4. Import/replace/delete/open actions as allowed

### 7.7 Read-only rules

The card drawer is never used for Standard Pack editing.

For Standard Pack card selection:

1. Either no drawer opens
2. Or a future dedicated read-only inspector opens

For v1 this specification chooses:

1. No editable drawer for Standard Pack
2. Standard Pack remains browse-only

### 7.8 API dependency status

| Capability | Status | Notes |
|---|---|---|
| Open card editor from card row | Future API required | Current app can list cards, but no explicit editable-card snapshot command exists in the active shell |
| Persist card updates | Partially available | Create/update/delete commands exist, but drawer-specific snapshot/load/unsaved workflow is not yet exposed as a dedicated UI contract |
| Card asset state | Future API required | Full drawer asset workflow is defined in long-term interface docs, not current app commands |
| Script/image actions | Future API required | Not currently exposed in the active command surface |

## 8. Modal Specifications

All utility flows in this section use centered modals.

Shared modal rules:

1. Modal appears above the main shell
2. Background shell remains visible but inactive
3. Modals are focus-trapped
4. Escape closes the modal when safe
5. If there are unsaved edits or staged warnings, close requires explicit handling
6. Modal body may scroll locally if needed
7. Modal opening must not shift the shell layout

## 8.1 `Workspace` modal

### Purpose

Manage program-level workspace operations without leaving the main shell.

### Sections

The modal contains these sections:

1. Recent workspaces
2. Open workspace by path
3. Create workspace

### Default state

The modal opens to the recent workspaces list.

### Recent workspaces section

Must show:

1. Workspace name
2. Path
3. Last opened timestamp

Actions per row:

1. Open

### Open workspace by path

Fields:

1. Workspace path

Action:

1. `Open Workspace`

### Create workspace

Fields:

1. Parent/root path
2. Workspace name
3. Description

Actions:

1. `Create`
2. `Create and Open`

### States

Required states:

1. Loading
2. Empty recent list
3. Validation error
4. Success feedback
5. Failure feedback

### API dependency status

| Capability | Status | Notes |
|---|---|---|
| List recent workspaces | Available now | Command exists |
| Open workspace by path | Available now | Command exists |
| Create workspace | Available now | Command exists |
| Remove record / delete directory | Out of scope for v1/P1 | Current backend still exposes `delete_workspace`, but the v1 UI will not implement workspace deletion or unregister flows |

## 8.2 `Global Settings` modal

### Purpose

Manage program-level configuration without leaving the main shell.

### Fields

1. App language
2. YGOPro path
3. External text editor path
4. Recommended code minimum
5. Recommended code maximum
6. Code minimum gap

### Actions

1. Save
2. Cancel
3. Optional reset-to-current if needed by implementation

### States

1. Loading
2. Editable
3. Validation error
4. Save success
5. Save failure

### API dependency status

| Capability | Status | Notes |
|---|---|---|
| Load config | Available now | Command exists |
| Save config | Available now | Command exists |
| Dedicated YGOPro path validation check | Future API required | Long-term design exists, but current command surface does not expose a dedicated path check |

## 8.3 `Export Expansions` modal

### Purpose

Export one or more selected packs into a runtime-style YGOPro resource bundle.

### Flow model

This is a full two-step modal flow:

1. Input step
2. Preview/issues step
3. Execute/result states

### Step 1: input

Required inputs:

1. Selected pack list
2. Export language
3. Output directory
4. Output name

Recommended layout:

1. Pack multi-select region
2. Language selector
3. Output path input
4. Output name input
5. Primary action: `Preview Export`

### Step 2: preview/issues

Must show:

1. Pack count
2. Card count
3. Main image count
4. Field image count
5. Script count
6. Warning count
7. Error count
8. Detailed issues list

Actions:

1. `Back`
2. `Export`

Rules:

1. Blocking errors disable `Export`
2. Warnings are visible and grouped
3. If preview becomes stale, the user must re-run preview

### Execute/result states

Must support:

1. Export in progress
2. Export succeeded
3. Export failed

Success result must show:

1. Output directory
2. Exported card count
3. Exported pack count
4. Exported assets summary

### API dependency status

| Capability | Status | Notes |
|---|---|---|
| Export modal shell | Future API required | No current export command surface is exposed in the app |
| Preview export | Future API required | Long-term interface design exists, current app does not expose it |
| Execute export | Future API required | Long-term interface design exists, current app does not expose it |

## 8.4 `+` modal

### Purpose

Create, open, or import packs without leaving the current shell context.

### Tabs

The modal contains exactly three tabs:

1. `Open Pack`
2. `Create Pack`
3. `Import Pack`

Default tab:

1. `Open Pack`

## 8.5 `Open Pack` tab

### Purpose

Open an unopened custom pack from the current workspace.

### Content

Must show the workspace's custom packs that are not already open.

Per row:

1. Pack name
2. Author
3. Version
4. Card count
5. Updated at
6. Open action

### States

1. Loading
2. Empty list
3. Open success
4. Open failure

### API dependency status

| Capability | Status | Notes |
|---|---|---|
| List pack overviews | Available now | Command exists |
| Open pack | Available now | Command exists |

## 8.6 `Create Pack` tab

### Purpose

Create a new empty custom pack inside the current workspace.

### Fields

1. Pack name
2. Author
3. Version
4. Description
5. Display language order
6. Default export language

### Actions

1. `Create Pack`
2. Optional `Create and Open` if the implementation wants immediate-open behavior

### States

1. No workspace open
   - Entire tab disabled with message
2. Editable
3. Validation error
4. Success
5. Failure

### API dependency status

| Capability | Status | Notes |
|---|---|---|
| Create pack | Available now | Command exists |
| Auto-open after create | Partially available | Create and open can be composed using existing commands, but not as a single explicit API |

## 8.7 `Import Pack` tab

### Purpose

Import runtime-style YGOPro resources into a new author-state pack.

### Flow model

This is a full two-step modal flow:

1. Input step
2. Preview/issues step
3. Execute/result states

### Step 1: input

Required fields:

1. New pack name
2. New pack author
3. New pack version
4. Display language order
5. Source language
6. Source `.cdb` path

Optional fields:

1. New pack description
2. Default export language
3. `pics/` path
4. `pics/field/` path
5. `script/` path
6. `strings.conf` path

Primary action:

1. `Preview Import`

### Step 2: preview/issues

Must show:

1. Target pack name
2. Card count
3. Warning count
4. Error count
5. Missing main image count
6. Missing script count
7. Missing field image count
8. Detailed issue list

Actions:

1. `Back`
2. `Import`

Rules:

1. Blocking errors disable `Import`
2. Preview is required before execute
3. If the preview becomes stale, the user must re-run preview

### Execute/result states

Must support:

1. Import in progress
2. Import succeeded
3. Import failed

Success result should show:

1. Imported card count
2. Imported target pack name
3. Missing resource summaries

### API dependency status

| Capability | Status | Notes |
|---|---|---|
| Import modal shell | Future API required | No current import command surface is exposed in the app |
| Preview import | Future API required | Long-term interface design exists, current app does not expose it |
| Execute import | Future API required | Long-term interface design exists, current app does not expose it |

## 9. Read-only vs Editable Surfaces

### 9.1 Custom pack

Custom pack surfaces are editable by default.

Editable surfaces include:

1. Pack metadata
2. Cards tab
3. Strings tab
4. Card editor drawer

### 9.2 Standard Pack

Standard Pack is read-only.

Read-only surfaces include:

1. Metadata summary
2. Card browsing
3. String browsing

Forbidden in Standard Pack:

1. Pack metadata editing
2. Card editing drawer
3. Card creation
4. String editing
5. Asset actions

## 10. Local Scroll Rules

To preserve one-screen shell usage, scrolling is constrained as follows:

1. The main window shell must not scroll
2. The sidebar pack list may scroll locally
3. The `Cards` tab body may scroll locally
4. The `Strings` tab body may scroll locally
5. Expanded metadata content may scroll locally if needed
6. Modal bodies may scroll locally
7. The card editor drawer body may scroll locally

## 11. Keyboard Expectations

Only obvious keyboard expectations are locked in v1:

1. `Escape`
   - closes modal when safe
   - closes drawer when safe
2. `Enter`
   - submits the focused primary form action when appropriate
3. `Tab`
   - follows normal focus order inside modals and drawer

No additional global shortcut system is required by this document.

## 12. API Support Matrix

This section summarizes the current implementation impact.

| UI capability | Status |
|---|---|
| Load/save global settings | Available now |
| List recent workspaces | Available now |
| Create/open workspace | Available now |
| List pack overviews | Available now |
| Create/open custom pack | Available now |
| Read active open-pack state | Available now |
| List cards for custom pack | Available now |
| Custom titlebar native controls | Available now |
| Close-pack command for UI | Partially available |
| Standard Pack read-only browsing | Future API required |
| Strings tab command surface | Future API required |
| Card drawer snapshot/load contract | Future API required |
| Card asset actions | Future API required |
| Export preview/execute | Future API required |
| Import preview/execute | Future API required |

## 13. Acceptance Scenarios

Implementation should be considered aligned with this specification only if all of the following are true:

1. The app opens into a single-screen desktop shell at `1280x800` without full-page scrollbars
2. The custom titlebar shows app icon, workspace name, and Windows-style controls
3. Clicking each top-left action button opens the correct centered modal
4. The left sidebar shows opened custom packs only, then `+`, then pinned `Standard Pack`
5. Clicking `Standard Pack` opens a read-only main-pane view
6. Clicking `+` opens a modal with `Open Pack`, `Create Pack`, and `Import Pack`
7. The collapsed metadata bar shows name/author/version/languages
8. `Cards` and `Strings` tabs switch within the same pack view
9. Single-clicking a card row opens the right overlay drawer
10. The drawer always keeps card image visible and uses internal tabs
11. The `Workspace` modal supports recent-workspace browsing, open-by-path, and create flows
12. The `Export Expansions` modal follows input -> preview -> execute flow
13. The `Import Pack` tab follows input -> preview -> execute flow

## 14. Non-Goals for This Document

This document does not define:

1. Final visual theme tokens
2. Icon artwork style
3. Animation durations beyond behavioral intent
4. macOS/Linux titlebar rules
5. Full i18n strategy
6. Internal frontend component file layout

These can be specified later as implementation details as long as they do not violate the structural and interaction rules in this document.
