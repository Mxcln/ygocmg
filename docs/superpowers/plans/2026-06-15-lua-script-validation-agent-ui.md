# Lua Script Validation Agent/UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose Lua script validation through a read-only Agent tool and a card asset bar validate button that opens a structured report dialog.

**Architecture:** Keep validation rules in the existing backend and `scriptApi.validateLuaScript` wrapper. Add thin frontend entry points: an Agent tool for conversational use, and a local modal dialog launched from `CardAssetBar` for direct user use.

**Tech Stack:** React 19, TypeScript, CSS modules, Vitest, React DOM server rendering for lightweight component assertions, existing YGOCMG i18n and API wrapper patterns.

---

## File Structure

- Create `src/features/agent/tools/scriptTools.ts`: read-only `validate_lua_script` tool, level validation, selected-card fallback.
- Create `src/features/agent/tools/scriptTools.test.ts`: TDD coverage for tool behavior and registry exposure.
- Modify `src/features/agent/tools/types.ts`: add `selectedCardId` to `ToolContext`.
- Modify `src/features/agent/tools/registry.ts`: register `validateLuaScriptTool`.
- Modify `src/features/agent/useAgentLoop.ts`: pass selected card id into tool context.
- Modify `src/features/agent/systemPrompt.ts`: teach the model when and how to use the validation tool.
- Create `src/features/card/ScriptValidationReportDialog.tsx`: report dialog and exported report content component.
- Create `src/features/card/ScriptValidationReportDialog.module.css`: modal/report styles.
- Create `src/features/card/ScriptValidationReportDialog.test.tsx`: server-render report content tests.
- Modify `src/features/card/CardAssetBar.tsx`: add validate button, loading state, API call, dialog state.
- Modify `src/shared/i18n/messages/en-US.ts`, `ja-JP.ts`, `zh-CN.ts`: add report labels.
- Modify docs: `docs/functional_spec.md`, `docs/system_architecture.md`, `docs/code_structure_api.md`, `docs/ui_design.md`, `docs/agent.md`.

## Task 1: Agent Tool Tests

**Files:**
- Create: `src/features/agent/tools/scriptTools.test.ts`
- Later modify: `src/features/agent/tools/scriptTools.ts`

- [ ] **Step 1: Write failing tests**

```ts
import { beforeEach, describe, expect, it, vi } from "vitest";
import { scriptApi } from "../../../shared/api/scriptApi";
import type { LuaValidationReport } from "../../../shared/contracts/script";
import { TOOL_DEFINITIONS, AGENT_TOOLS } from "./registry";
import { validateLuaScriptTool } from "./scriptTools";
import type { ToolContext } from "./types";

vi.mock("../../../shared/api/scriptApi", () => ({
  scriptApi: {
    validateLuaScript: vi.fn(),
  },
}));

const baseCtx: ToolContext = {
  workspaceId: "workspace-1",
  packId: "pack-1",
  selectedCardId: "selected-card",
};

const report: LuaValidationReport = {
  status: "pass",
  confidence: "high",
  summary: "Script loaded.",
  issues: [],
  stages: [],
  limitations: [],
};

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(scriptApi.validateLuaScript).mockResolvedValue(report);
});

describe("validateLuaScriptTool", () => {
  it("rejects missing active pack before calling the API", async () => {
    await expect(
      validateLuaScriptTool.execute({}, { workspaceId: null, packId: null, selectedCardId: null }),
    ).rejects.toThrow("No active pack");
    expect(scriptApi.validateLuaScript).not.toHaveBeenCalled();
  });

  it("requires a card id when no selected card is available", async () => {
    await expect(
      validateLuaScriptTool.execute({}, { ...baseCtx, selectedCardId: null }),
    ).rejects.toThrow("Specify a card id or open a card in the editor");
    expect(scriptApi.validateLuaScript).not.toHaveBeenCalled();
  });

  it("uses the selected card when cardId is omitted", async () => {
    await expect(validateLuaScriptTool.execute({}, baseCtx)).resolves.toBe(report);
    expect(scriptApi.validateLuaScript).toHaveBeenCalledWith({
      workspaceId: "workspace-1",
      packId: "pack-1",
      cardId: "selected-card",
    });
  });

  it("prefers an explicit cardId over the selected card", async () => {
    await validateLuaScriptTool.execute({ cardId: "explicit-card" }, baseCtx);
    expect(scriptApi.validateLuaScript).toHaveBeenCalledWith({
      workspaceId: "workspace-1",
      packId: "pack-1",
      cardId: "explicit-card",
    });
  });

  it("forwards valid explicit levels", async () => {
    await validateLuaScriptTool.execute(
      { cardId: "card-1", levels: ["static", "ocgcore_init"] },
      baseCtx,
    );
    expect(scriptApi.validateLuaScript).toHaveBeenCalledWith({
      workspaceId: "workspace-1",
      packId: "pack-1",
      cardId: "card-1",
      levels: ["static", "ocgcore_init"],
    });
  });

  it("rejects invalid levels before calling the API", async () => {
    await expect(
      validateLuaScriptTool.execute({ levels: ["static", "made_up"] }, baseCtx),
    ).rejects.toThrow("Unsupported validation level");
    expect(scriptApi.validateLuaScript).not.toHaveBeenCalled();
  });
});

describe("script validation tool registry", () => {
  it("registers validate_lua_script as an agent tool", () => {
    expect(AGENT_TOOLS.some((tool) => tool.name === "validate_lua_script")).toBe(true);
  });

  it("includes validate_lua_script in model tool definitions", () => {
    expect(
      TOOL_DEFINITIONS.some((definition) => definition.function.name === "validate_lua_script"),
    ).toBe(true);
  });
});
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```powershell
npm test -- src/features/agent/tools/scriptTools.test.ts
```

Expected: fail because `scriptTools.ts` and `selectedCardId` do not exist.

## Task 2: Agent Tool Implementation

**Files:**
- Create: `src/features/agent/tools/scriptTools.ts`
- Modify: `src/features/agent/tools/types.ts`
- Modify: `src/features/agent/tools/registry.ts`
- Modify: `src/features/agent/useAgentLoop.ts`
- Modify: `src/features/agent/systemPrompt.ts`

- [ ] **Step 1: Extend `ToolContext`**

Add to `src/features/agent/tools/types.ts`:

```ts
export interface ToolContext {
  workspaceId: string | null;
  packId: string | null;
  selectedCardId: string | null;
}
```

- [ ] **Step 2: Add the tool**

Create `src/features/agent/tools/scriptTools.ts`:

```ts
import { scriptApi } from "../../../shared/api/scriptApi";
import type { LuaValidationLevel } from "../../../shared/contracts/script";
import type { AgentTool } from "./types";
import { requirePack, ToolError } from "./types";

const VALID_LEVELS = new Set<LuaValidationLevel>([
  "static",
  "ocgcore_init",
  "smoke",
  "scenario",
]);

function normalizeCardId(value: unknown): string | null {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

function normalizeLevels(value: unknown): LuaValidationLevel[] | undefined {
  if (value === undefined) return undefined;
  if (!Array.isArray(value)) {
    throw new ToolError("levels must be an array of validation level strings.");
  }
  const levels = value.map((item) => String(item));
  const invalid = levels.find((level) => !VALID_LEVELS.has(level as LuaValidationLevel));
  if (invalid) {
    throw new ToolError(
      `Unsupported validation level "${invalid}". Use static, ocgcore_init, smoke, or scenario.`,
    );
  }
  return levels as LuaValidationLevel[];
}

export const validateLuaScriptTool: AgentTool = {
  name: "validate_lua_script",
  description:
    "Validate a custom card's saved Lua script in the active pack. This read-only tool " +
    "returns the backend LuaValidationReport with static and ocgcore_init results by default. " +
    "It checks script load/init only and does not edit or fix scripts.",
  readOnly: true,
  parameters: {
    type: "object",
    properties: {
      cardId: {
        type: "string",
        description:
          "Optional card id. Omit to validate the current Selected card from the editor.",
      },
      levels: {
        type: "array",
        items: {
          type: "string",
          enum: ["static", "ocgcore_init", "smoke", "scenario"],
        },
        description:
          "Optional validation levels. Omit to use the backend default static + ocgcore_init.",
      },
    },
  },
  async execute(args, ctx) {
    const { workspaceId, packId } = requirePack(ctx);
    const cardId = normalizeCardId(args.cardId) ?? ctx.selectedCardId;
    if (!cardId) {
      throw new ToolError(
        "Specify a card id or open a card in the editor before validating its Lua script.",
      );
    }
    const levels = normalizeLevels(args.levels);
    return scriptApi.validateLuaScript({
      workspaceId,
      packId,
      cardId,
      ...(levels ? { levels } : {}),
    });
  },
};
```

- [ ] **Step 3: Register the tool**

Modify `src/features/agent/tools/registry.ts`:

```ts
import { validateLuaScriptTool } from "./scriptTools";
```

Add it after existing read-only tools in `AGENT_TOOLS`.

- [ ] **Step 4: Pass selected card id into context**

Modify `src/features/agent/useAgentLoop.ts`:

```ts
const ctx: ToolContext = {
  workspaceId: shell.workspaceId,
  packId: shell.activePackId,
  selectedCardId: shell.selectedCard?.id ?? null,
};
```

Update existing tests or test contexts that construct `ToolContext` to include `selectedCardId: null`.

- [ ] **Step 5: Update the system prompt**

Add to `src/features/agent/systemPrompt.ts` under discovering/current-selection guidance:

```ts
- Use validate_lua_script when the user asks to validate, check, test, or diagnose a card's Lua script. If the user says "this card", prefer the Selected card. The tool is read-only: it reports validation results but does not edit or fix scripts. Its ocgcore_init stage proves the script loads and initial_effect can run, not that the card effect is semantically correct in a duel.
```

- [ ] **Step 6: Run tests to verify GREEN**

Run:

```powershell
npm test -- src/features/agent/tools/scriptTools.test.ts src/features/agent/tools/writeTools.test.ts
```

Expected: pass.

## Task 3: Report Dialog Tests

**Files:**
- Create: `src/features/card/ScriptValidationReportDialog.test.tsx`
- Later create: `src/features/card/ScriptValidationReportDialog.tsx`

- [ ] **Step 1: Write failing render tests**

```tsx
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import type { LuaValidationReport } from "../../shared/contracts/script";
import { ScriptValidationReportContent } from "./ScriptValidationReportDialog";

const t = (id: string, values?: Record<string, string | number>) =>
  values
    ? `${id} ${Object.entries(values)
        .map(([key, value]) => `${key}=${value}`)
        .join(" ")}`
    : id;

const report: LuaValidationReport = {
  status: "inconclusive",
  confidence: "medium",
  summary: "ocgcore helper was not available.",
  stages: [
    {
      stage: "static",
      status: "pass",
      durationMs: 3,
      issues: [],
    },
    {
      stage: "ocgcore_init",
      status: "inconclusive",
      durationMs: 12,
      issues: [
        {
          severity: "warning",
          stage: "ocgcore_init",
          code: "helper_not_found",
          message: "Helper executable was not found.",
          line: 8,
          column: 2,
          suggestion: "Build the helper before running ocgcore validation.",
        },
      ],
    },
  ],
  issues: [
    {
      severity: "warning",
      stage: "ocgcore_init",
      code: "helper_not_found",
      message: "Helper executable was not found.",
      line: 8,
      column: 2,
      suggestion: "Build the helper before running ocgcore validation.",
    },
  ],
  limitations: ["ocgcore_init does not prove effect semantics."],
};

describe("ScriptValidationReportContent", () => {
  it("renders summary, status, stages, issues, locations, suggestions, and limitations", () => {
    const html = renderToStaticMarkup(<ScriptValidationReportContent report={report} t={t} />);

    expect(html).toContain("inconclusive");
    expect(html).toContain("ocgcore helper was not available.");
    expect(html).toContain("static");
    expect(html).toContain("ocgcore_init");
    expect(html).toContain("helper_not_found");
    expect(html).toContain("Helper executable was not found.");
    expect(html).toContain("line=8");
    expect(html).toContain("column=2");
    expect(html).toContain("Build the helper");
    expect(html).toContain("ocgcore_init does not prove effect semantics.");
  });

  it("renders explicit empty states for reports without issues or limitations", () => {
    const html = renderToStaticMarkup(
      <ScriptValidationReportContent
        report={{ ...report, status: "pass", issues: [], limitations: [] }}
        t={t}
      />,
    );

    expect(html).toContain("card.scriptValidation.noIssues");
    expect(html).toContain("card.scriptValidation.noLimitations");
  });
});
```

- [ ] **Step 2: Run tests to verify RED**

Run:

```powershell
npm test -- src/features/card/ScriptValidationReportDialog.test.tsx
```

Expected: fail because the dialog file does not exist.

## Task 4: Report Dialog Implementation

**Files:**
- Create: `src/features/card/ScriptValidationReportDialog.tsx`
- Create: `src/features/card/ScriptValidationReportDialog.module.css`
- Modify: i18n locale files

- [ ] **Step 1: Implement dialog and content component**

Create `src/features/card/ScriptValidationReportDialog.tsx` with:

```tsx
import type { LuaValidationIssue, LuaValidationReport } from "../../shared/contracts/script";
import { useAppI18n } from "../../shared/i18n";
import shared from "../../shared/styles/shared.module.css";
import styles from "./ScriptValidationReportDialog.module.css";

type Translate = (id: string, values?: Record<string, string | number>) => string;

interface ScriptValidationReportDialogProps {
  report: LuaValidationReport | null;
  open: boolean;
  onClose: () => void;
}

function issueLocation(issue: LuaValidationIssue, t: Translate): string | null {
  const parts: string[] = [];
  if (issue.line !== undefined) parts.push(t("card.scriptValidation.line", { line: issue.line }));
  if (issue.column !== undefined) {
    parts.push(t("card.scriptValidation.column", { column: issue.column }));
  }
  return parts.length ? parts.join(" ") : null;
}

export function ScriptValidationReportContent({
  report,
  t,
}: {
  report: LuaValidationReport;
  t: Translate;
}) {
  return (
    <div className={styles.reportContent}>
      <section className={styles.summaryBlock} data-status={report.status}>
        <div>
          <span className={styles.kicker}>{t("card.scriptValidation.status")}</span>
          <strong>{report.status}</strong>
        </div>
        <div>
          <span className={styles.kicker}>{t("card.scriptValidation.confidence")}</span>
          <strong>{report.confidence}</strong>
        </div>
        <p>{report.summary}</p>
      </section>

      <section className={styles.section}>
        <h3>{t("card.scriptValidation.stages")}</h3>
        <div className={styles.stageList}>
          {report.stages.map((stage) => (
            <div className={styles.stageRow} data-status={stage.status} key={stage.stage}>
              <strong>{stage.stage}</strong>
              <span>{stage.status}</span>
              <span>{t("card.scriptValidation.durationMs", { duration: stage.durationMs })}</span>
              <span>{t("card.scriptValidation.issueCount", { count: stage.issues.length })}</span>
            </div>
          ))}
        </div>
      </section>

      <section className={styles.section}>
        <h3>{t("card.scriptValidation.issues")}</h3>
        {report.issues.length ? (
          <ul className={styles.issueList}>
            {report.issues.map((issue, index) => {
              const location = issueLocation(issue, t);
              return (
                <li className={styles.issueItem} data-severity={issue.severity} key={`${issue.code}-${index}`}>
                  <div className={styles.issueHead}>
                    <strong>{issue.code}</strong>
                    <span>{issue.severity}</span>
                    <span>{issue.stage}</span>
                    {location && <span>{location}</span>}
                  </div>
                  <p>{issue.message}</p>
                  {issue.suggestion && (
                    <p className={styles.suggestion}>
                      {t("card.scriptValidation.suggestion")}: {issue.suggestion}
                    </p>
                  )}
                </li>
              );
            })}
          </ul>
        ) : (
          <p className={styles.emptyText}>{t("card.scriptValidation.noIssues")}</p>
        )}
      </section>

      <section className={styles.section}>
        <h3>{t("card.scriptValidation.limitations")}</h3>
        {report.limitations.length ? (
          <ul className={styles.limitList}>
            {report.limitations.map((limitation, index) => (
              <li key={`${limitation}-${index}`}>{limitation}</li>
            ))}
          </ul>
        ) : (
          <p className={styles.emptyText}>{t("card.scriptValidation.noLimitations")}</p>
        )}
      </section>
    </div>
  );
}

export function ScriptValidationReportDialog({
  report,
  open,
  onClose,
}: ScriptValidationReportDialogProps) {
  const { t } = useAppI18n();
  if (!open || !report) return null;

  return (
    <div className={styles.dialogLayer}>
      <div className={styles.dialogBackdrop} onClick={onClose} />
      <section
        className={styles.dialogBox}
        role="dialog"
        aria-modal="true"
        aria-labelledby="script-validation-report-title"
      >
        <header className={shared.modalHeader}>
          <h2 id="script-validation-report-title">{t("card.scriptValidation.title")}</h2>
          <button type="button" className={shared.modalCloseButton} onClick={onClose}>
            {t("action.close")}
          </button>
        </header>
        <div className={shared.modalBody}>
          <ScriptValidationReportContent report={report} t={t} />
        </div>
      </section>
    </div>
  );
}
```

- [ ] **Step 2: Add CSS**

Create `src/features/card/ScriptValidationReportDialog.module.css` with modal layout, wrapping rows, status badges, and scroll-safe body.

- [ ] **Step 3: Add i18n keys**

Add keys in all locale files:

```ts
"card.scriptValidation.title": "...",
"card.scriptValidation.validate": "...",
"card.scriptValidation.validating": "...",
"card.scriptValidation.status": "...",
"card.scriptValidation.confidence": "...",
"card.scriptValidation.stages": "...",
"card.scriptValidation.issues": "...",
"card.scriptValidation.limitations": "...",
"card.scriptValidation.durationMs": "{duration} ms",
"card.scriptValidation.issueCount": "{count} issues",
"card.scriptValidation.line": "Line {line}",
"card.scriptValidation.column": "Column {column}",
"card.scriptValidation.suggestion": "Suggestion",
"card.scriptValidation.noIssues": "...",
"card.scriptValidation.noLimitations": "...",
```

- [ ] **Step 4: Run tests to verify GREEN**

Run:

```powershell
npm test -- src/features/card/ScriptValidationReportDialog.test.tsx
```

Expected: pass.

## Task 5: Card Asset Bar Integration

**Files:**
- Modify: `src/features/card/CardAssetBar.tsx`
- Modify: `src/features/card/CardAssetBar.module.css`

- [ ] **Step 1: Import script API, report type, and dialog**

Add:

```ts
import { scriptApi } from "../../shared/api/scriptApi";
import type { LuaValidationReport } from "../../shared/contracts/script";
import { ScriptValidationReportDialog } from "./ScriptValidationReportDialog";
```

- [ ] **Step 2: Add local validation state**

Inside `CardAssetBar`:

```ts
const [validatingScript, setValidatingScript] = useState(false);
const [validationReport, setValidationReport] = useState<LuaValidationReport | null>(null);
const [validationDialogOpen, setValidationDialogOpen] = useState(false);
const busyAny = busy || validatingScript;
```

Use `busyAny` for resource button disabled checks.

- [ ] **Step 3: Add handler**

```ts
async function handleValidateScript() {
  if (isCreate || !cardId) return;
  setValidatingScript(true);
  try {
    const report = await scriptApi.validateLuaScript({ workspaceId, packId, cardId });
    setValidationReport(report);
    setValidationDialogOpen(true);
  } catch (err) {
    onError(formatError(err));
  } finally {
    setValidatingScript(false);
  }
}
```

- [ ] **Step 4: Add validate button**

Inside the `assetState.has_script` button group, add:

```tsx
<button
  type="button"
  className={styles.assetSegBtn}
  disabled={isCreate || busyAny}
  onClick={() => void handleValidateScript()}
>
  {validatingScript
    ? t("card.scriptValidation.validating")
    : t("card.scriptValidation.validate")}
</button>
```

- [ ] **Step 5: Render dialog**

At the end of the returned JSX:

```tsx
<ScriptValidationReportDialog
  report={validationReport}
  open={validationDialogOpen}
  onClose={() => setValidationDialogOpen(false)}
/>
```

- [ ] **Step 6: Typecheck**

Run:

```powershell
npm run typecheck
```

Expected: pass or expose issues to fix immediately.

## Task 6: Documentation Updates

**Files:**
- Modify: `docs/functional_spec.md`
- Modify: `docs/system_architecture.md`
- Modify: `docs/code_structure_api.md`
- Modify: `docs/ui_design.md`
- Modify: `docs/agent.md`

- [ ] **Step 1: Update functional facts**

Document that custom pack script validation is available through both the Agent and the card resource UI report dialog.

- [ ] **Step 2: Update architecture**

Replace the stale “Agent/UI entry points remain future work” sentence with current facts: Agent and UI are thin callers over `scriptApi.validateLuaScript`.

- [ ] **Step 3: Update code structure/API docs**

Mention `src/features/agent/tools/scriptTools.ts` and `src/features/card/ScriptValidationReportDialog.tsx`.

- [ ] **Step 4: Update UI docs**

Mention the `CardAssetBar` validate button and explicit report dialog.

- [ ] **Step 5: Update agent docs**

Add `validate_lua_script` to the read-only tool list and document selected-card fallback and limitations.

- [ ] **Step 6: Search for stale claims**

Run:

```powershell
rg "尚未调用 helper|不运行 ocgcore|当前阶段返回静态检查报告|Agent/UI 入口仍属于后续阶段" docs
```

Expected: no stale current-fact claims remain. Hits inside historical reports are acceptable only when clearly historical.

## Task 7: Full Verification

**Files:** no production edits expected.

- [ ] **Step 1: Run frontend unit tests**

```powershell
npm test -- src/shared/api/scriptApi.test.ts src/features/agent/tools/scriptTools.test.ts src/features/agent/tools/writeTools.test.ts src/features/card/ScriptValidationReportDialog.test.tsx
```

Expected: pass.

- [ ] **Step 2: Run frontend typecheck**

```powershell
npm run typecheck
```

Expected: pass.

- [ ] **Step 3: Run Rust script tests**

```powershell
Push-Location src-tauri
cargo test application::script
cargo test infrastructure::ocgcore_validator
cargo check
Pop-Location
```

Expected: pass.

- [ ] **Step 4: Review git diff**

```powershell
git status --short
git diff --stat
```

Expected: only intended phase 7/8 files changed.
