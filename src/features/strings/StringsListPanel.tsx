import { useCallback, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useShellStore } from "../../shared/stores/shellStore";
import { stringsApi } from "../../shared/api/stringsApi";
import { formatError, formatStringKeyHex } from "../../shared/utils/format";
import type { TextLanguageProfile } from "../../shared/contracts/config";
import type { PackStringEntry, PackStringsPage } from "../../shared/contracts/strings";
import type { ValidationIssue } from "../../shared/contracts/common";
import { useAppI18n } from "../../shared/i18n";
import shared from "../../shared/styles/shared.module.css";
import { StringsBrowserPanel } from "./StringsBrowserPanel";
import type { StringsBrowserQuery } from "./StringsBrowserPanel";

export function StringsListPanel({ catalog }: { catalog: TextLanguageProfile[] }) {
  const { t, td } = useAppI18n();
  const workspaceId = useShellStore((s) => s.workspaceId);
  const activePackId = useShellStore((s) => s.activePackId);
  const activeMeta = useShellStore((s) =>
    s.activePackId ? s.packMetadataMap[s.activePackId] : null,
  );
  const openDialog = useShellStore((s) => s.openDialog);
  const closeDialog = useShellStore((s) => s.closeDialog);
  const queryClient = useQueryClient();
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const languages = activeMeta?.display_language_order ?? [];
  const enabled = !!workspaceId && !!activePackId && languages.length > 0;

  const invalidateStrings = useCallback(() => {
    void queryClient.invalidateQueries({ queryKey: ["strings"] });
  }, [queryClient]);

  function openWarningsDialog(
    warnings: ValidationIssue[],
    onConfirm: () => Promise<void>,
  ) {
    openDialog({
      kind: "warning",
      title: td("strings.warning.title", "Review string warnings"),
      message: td("strings.warning.message", "This string change produced warnings. Continue to apply it?"),
      confirmLabel: td("action.apply", "Apply"),
      cancelLabel: t("action.cancel"),
      warnings,
      onConfirm,
    });
  }

  async function loadPage(query: StringsBrowserQuery): Promise<PackStringsPage> {
    return stringsApi.listPackStrings({
      workspaceId: workspaceId!,
      packId: activePackId!,
      language: query.language,
      kindFilter: query.kindFilter || null,
      keyFilter: query.keyFilter,
      keyword: query.keyword || null,
      page: query.page,
      pageSize: query.pageSize,
    });
  }

  async function commitEntry(entry: PackStringEntry, language: string) {
    if (!workspaceId || !activePackId || saving) return;
    setSaving(true);
    setErrorMsg(null);
    try {
      const result = await stringsApi.upsertPackString({
        workspaceId,
        packId: activePackId,
        language,
        entry,
      });

      if (result.status === "ok") {
        invalidateStrings();
      } else {
        openWarningsDialog(result.warnings, async () => {
          try {
            await stringsApi.confirmPackStringsWrite({
              confirmationToken: result.confirmation_token,
            });
            closeDialog();
            invalidateStrings();
          } catch (err) {
            setErrorMsg(formatError(err));
          }
        });
      }
    } catch (err) {
      setErrorMsg(formatError(err));
    } finally {
      setSaving(false);
    }
  }

  async function clearTranslation(entry: PackStringEntry, language: string) {
    if (!workspaceId || !activePackId || saving) return;
    setSaving(true);
    setErrorMsg(null);
    try {
      await stringsApi.removePackStringTranslation({
        workspaceId,
        packId: activePackId,
        kind: entry.kind,
        key: entry.key,
        language,
      });
      invalidateStrings();
    } catch (err) {
      setErrorMsg(formatError(err));
    } finally {
      setSaving(false);
    }
  }

  function handleDeleteEntry(entry: PackStringEntry) {
    if (!workspaceId || !activePackId) return;
    openDialog({
      kind: "confirm",
      title: td("strings.delete.title", "Delete string entry"),
      message: td("strings.delete.message", "Delete {kind}[{key}]? This cannot be undone.", {
        kind: entry.kind,
        key: formatStringKeyHex(entry.key),
      }),
      confirmLabel: t("action.delete"),
      cancelLabel: t("action.cancel"),
      danger: true,
      onConfirm: async () => {
        try {
          await stringsApi.deletePackStrings({
            workspaceId,
            packId: activePackId,
            entries: [{ kind: entry.kind, key: entry.key }],
          });
          closeDialog();
          invalidateStrings();
        } catch (err) {
          setErrorMsg(formatError(err));
        }
      },
    });
  }

  if (languages.length === 0) {
    return (
      <div className={shared.cardListEmpty}>
        <p>{td("strings.noConfiguredLanguages", "No languages configured for this pack.")}</p>
        <p>{td("strings.addDisplayLanguagesHint", "Edit pack metadata to add display languages.")}</p>
      </div>
    );
  }

  return (
    <StringsBrowserPanel
      enabled={enabled}
      queryKeyBase={["strings", activePackId]}
      languages={languages}
      catalog={catalog}
      loadPage={loadPage}
      editable
      saving={saving}
      errorMessage={errorMsg}
      onCreate={commitEntry}
      onUpdate={commitEntry}
      onClearTranslation={clearTranslation}
      onDelete={handleDeleteEntry}
      emptyTitle={td("strings.emptyTitle", "No string entries yet.")}
      emptyHint={td("strings.emptyHint", "Click \"+ New String\" to create one.")}
    />
  );
}
