import { createElement } from "react";
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
    const html = renderToStaticMarkup(
      createElement(ScriptValidationReportContent, { report, t }),
    );

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
      createElement(ScriptValidationReportContent, {
        report: { ...report, status: "pass", issues: [], limitations: [] },
        t,
      }),
    );

    expect(html).toContain("card.scriptValidation.noIssues");
    expect(html).toContain("card.scriptValidation.noLimitations");
  });
});
