import { useState, useRef, useEffect, useCallback } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useShellStore } from "../../shared/stores/shellStore";
import { stringsApi } from "../../shared/api/stringsApi";
import { formatError, formatStringKeyHex, parseHexInput } from "../../shared/utils/format";
import type { PackStringKind, PackStringEntry, PackStringsPage } from "../../shared/contracts/strings";
import type { ValidationIssue } from "../../shared/contracts/common";

const PAGE_SIZE = 50;

const KIND_OPTIONS: { value: PackStringKind | ""; label: string }[] = [
  { value: "", label: "All Kinds" },
  { value: "counter", label: "Counter" },
  { value: "victory", label: "Victory" },
  { value: "setname", label: "Setname" },
];

interface EditingCell {
  kind: PackStringKind;
  key: number;
  value: string;
}

interface NewRow {
  kind: PackStringKind;
  key: string;
  value: string;
}

function normalizeHexDraft(value: string): string {
  const trimmed = value.trim();
  const withoutPrefix = trimmed.replace(/^0x/i, "");
  return withoutPrefix.replace(/[^0-9a-fA-F]/g, "").toUpperCase();
}

export function StringsListPanel() {
  const workspaceId = useShellStore((s) => s.workspaceId);
  const activePackId = useShellStore((s) => s.activePackId);
  const activeMeta = useShellStore((s) =>
    s.activePackId ? s.packMetadataMap[s.activePackId] : null,
  );
  const openDialog = useShellStore((s) => s.openDialog);
  const closeDialog = useShellStore((s) => s.closeDialog);
  const queryClient = useQueryClient();

  const languages = activeMeta?.display_language_order ?? [];
  const [language, setLanguage] = useState(languages[0] ?? "");
  const [kindFilter, setKindFilter] = useState<PackStringKind | "">("");
  const [keyword, setKeyword] = useState("");
  const [page, setPage] = useState(1);
  const [editingCell, setEditingCell] = useState<EditingCell | null>(null);
  const [newRow, setNewRow] = useState<NewRow | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const editInputRef = useRef<HTMLInputElement>(null);
  const newKeyRef = useRef<HTMLInputElement>(null);
  const previousHadNewRow = useRef(false);

  useEffect(() => {
    if (languages.length > 0 && !languages.includes(language)) {
      setLanguage(languages[0]);
    }
  }, [languages, language]);

  const enabled = !!workspaceId && !!activePackId && !!language;
  const queryKey = ["strings", activePackId, language, kindFilter, keyword, page];

  const { data, isLoading, error: queryError } = useQuery<PackStringsPage>({
    queryKey,
    queryFn: () =>
      stringsApi.listPackStrings({
        workspaceId: workspaceId!,
        packId: activePackId!,
        language,
        kindFilter: kindFilter || null,
        keyFilter: null,
        keyword: keyword || null,
        page,
        pageSize: PAGE_SIZE,
      }),
    enabled,
  });

  const items = data?.items ?? [];
  const total = data?.total ?? 0;
  const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE));

  useEffect(() => {
    if (editInputRef.current) editInputRef.current.focus();
  }, [editingCell]);

  useEffect(() => {
    if (newRow && !previousHadNewRow.current && newKeyRef.current) {
      newKeyRef.current.focus();
    }
    previousHadNewRow.current = newRow !== null;
  }, [newRow]);

  const invalidateStrings = useCallback(() => {
    void queryClient.invalidateQueries({ queryKey: ["strings"] });
  }, [queryClient]);

  function openWarningsDialog(
    warnings: ValidationIssue[],
    onConfirm: () => Promise<void>,
  ) {
    openDialog({
      kind: "warning",
      title: "Review string warnings",
      message: "This string change produced warnings. Continue to apply it?",
      confirmLabel: "Apply",
      cancelLabel: "Cancel",
      warnings,
      onConfirm,
    });
  }

  function handleStartEdit(entry: PackStringEntry) {
    if (saving) return;
    setEditingCell({ kind: entry.kind, key: entry.key, value: entry.value });
    setNewRow(null);
    setErrorMsg(null);
  }

  async function handleCommitEdit() {
    if (!editingCell || !workspaceId || !activePackId || saving) return;

    const original = items.find(
      (e) => e.kind === editingCell.kind && e.key === editingCell.key,
    );
    if (original && original.value === editingCell.value) {
      setEditingCell(null);
      return;
    }

    setSaving(true);
    setErrorMsg(null);
    try {
      const result = await stringsApi.upsertPackString({
        workspaceId,
        packId: activePackId,
        language,
        entry: {
          kind: editingCell.kind,
          key: editingCell.key,
          value: editingCell.value,
        },
      });

      if (result.status === "ok") {
        setEditingCell(null);
        invalidateStrings();
      } else {
        openWarningsDialog(result.warnings, async () => {
          try {
            await stringsApi.confirmPackStringsWrite({
              confirmationToken: result.confirmation_token,
            });
            closeDialog();
            setEditingCell(null);
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

  function handleCancelEdit() {
    setEditingCell(null);
  }

  function handleEditKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      e.stopPropagation();
      void handleCommitEdit();
    } else if (e.key === "Escape") {
      e.stopPropagation();
      handleCancelEdit();
    }
  }

  function handleStartNew() {
    setNewRow({ kind: "counter", key: "", value: "" });
    setEditingCell(null);
    setErrorMsg(null);
  }

  async function handleCommitNew() {
    if (!newRow || !workspaceId || !activePackId || saving) return;
    const parsedKey = parseHexInput(newRow.key);
    if (isNaN(parsedKey) || parsedKey < 0) {
      setErrorMsg("Key must be a non-negative hexadecimal value.");
      return;
    }
    if (!newRow.value.trim()) {
      setErrorMsg("Value cannot be empty.");
      return;
    }

    setSaving(true);
    setErrorMsg(null);
    try {
      const result = await stringsApi.upsertPackString({
        workspaceId,
        packId: activePackId,
        language,
        entry: { kind: newRow.kind, key: parsedKey, value: newRow.value },
      });

      if (result.status === "ok") {
        setNewRow(null);
        invalidateStrings();
      } else {
        openWarningsDialog(result.warnings, async () => {
          try {
            await stringsApi.confirmPackStringsWrite({
              confirmationToken: result.confirmation_token,
            });
            closeDialog();
            setNewRow(null);
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

  function handleNewKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      e.stopPropagation();
      void handleCommitNew();
    } else if (e.key === "Escape") {
      e.stopPropagation();
      setNewRow(null);
    }
  }

  function handleDeleteEntry(entry: PackStringEntry) {
    if (!workspaceId || !activePackId) return;
    openDialog({
      kind: "confirm",
      title: "Delete string entry",
      message: `Delete ${entry.kind}[${formatStringKeyHex(entry.key)}]? This cannot be undone.`,
      confirmLabel: "Delete",
      cancelLabel: "Cancel",
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

  if (!language && languages.length === 0) {
    return (
      <div className="card-list-empty">
        <p>No languages configured for this pack.</p>
        <p>Edit pack metadata to add display languages.</p>
      </div>
    );
  }

  return (
    <>
      <div className="strings-toolbar">
        <select
          className="strings-lang-select"
          value={language}
          onChange={(e) => {
            setLanguage(e.target.value);
            setPage(1);
            setEditingCell(null);
            setNewRow(null);
          }}
        >
          {languages.map((lang) => (
            <option key={lang} value={lang}>
              {lang}
            </option>
          ))}
        </select>
        <select
          className="strings-kind-select"
          value={kindFilter}
          onChange={(e) => {
            setKindFilter(e.target.value as PackStringKind | "");
            setPage(1);
          }}
        >
          {KIND_OPTIONS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
        <input
          className="strings-search-input"
          type="text"
          placeholder="Search value..."
          value={keyword}
          onChange={(e) => {
            setKeyword(e.target.value);
            setPage(1);
          }}
        />
        <button type="button" className="primary-button" onClick={handleStartNew}>
          + New String
        </button>
      </div>

      {errorMsg && <div className="strings-error">{errorMsg}</div>}

      {isLoading && items.length === 0 ? (
        <div className="card-list-empty">
          <p>Loading strings...</p>
        </div>
      ) : queryError ? (
        <div className="card-list-empty">
          <p>Failed to load strings.</p>
        </div>
      ) : items.length === 0 && !newRow ? (
        <div className="card-list-empty">
          <p>No string entries yet.</p>
          <p>Click &quot;+ New String&quot; to create one.</p>
        </div>
      ) : (
        <>
          <div className="strings-table-header">
            <span>Kind</span>
            <span>Key</span>
            <span>Value</span>
            <span />
          </div>
          <div className="strings-table-body">
            {newRow && (
              <div className="strings-table-row strings-new-row">
                <select
                  className="strings-cell-input strings-cell-kind"
                  value={newRow.kind}
                  onChange={(e) =>
                    setNewRow({ ...newRow, kind: e.target.value as PackStringKind })
                  }
                >
                  <option value="counter">Counter</option>
                  <option value="victory">Victory</option>
                  <option value="setname">Setname</option>
                </select>
                <input
                  ref={newKeyRef}
                  className="strings-cell-input strings-cell-key"
                  type="text"
                  inputMode="text"
                  placeholder="Hex key"
                  value={newRow.key}
                  onChange={(e) =>
                    setNewRow({ ...newRow, key: normalizeHexDraft(e.target.value) })
                  }
                  onKeyDown={handleNewKeyDown}
                />
                <input
                  className="strings-cell-input strings-cell-value"
                  type="text"
                  placeholder="Value"
                  value={newRow.value}
                  onChange={(e) => setNewRow({ ...newRow, value: e.target.value })}
                  onKeyDown={handleNewKeyDown}
                />
                <span className="strings-row-actions">
                  <button
                    type="button"
                    className="strings-action-btn strings-save-btn"
                    onClick={() => void handleCommitNew()}
                    disabled={saving}
                    title="Save"
                  >
                    OK
                  </button>
                  <button
                    type="button"
                    className="strings-action-btn"
                    onClick={() => setNewRow(null)}
                    title="Cancel"
                  >
                    X
                  </button>
                </span>
              </div>
            )}

            {items.map((entry) => {
              const isEditing =
                editingCell &&
                editingCell.kind === entry.kind &&
                editingCell.key === entry.key;

              return (
                <div
                  key={`${entry.kind}-${entry.key}`}
                  className={`strings-table-row ${isEditing ? "editing" : ""}`}
                >
                  <span className="strings-cell-kind-display">{entry.kind}</span>
                  <span className="strings-cell-key-display">{formatStringKeyHex(entry.key)}</span>
                  {isEditing ? (
                    <input
                      ref={editInputRef}
                      className="strings-cell-input strings-cell-value"
                      type="text"
                      value={editingCell.value}
                      onChange={(e) =>
                        setEditingCell({ ...editingCell, value: e.target.value })
                      }
                      onKeyDown={handleEditKeyDown}
                      onBlur={() => void handleCommitEdit()}
                    />
                  ) : (
                    <span
                      className="strings-cell-value-display"
                      onDoubleClick={() => handleStartEdit(entry)}
                      title="Double-click to edit"
                    >
                      {entry.value || "\u00A0"}
                    </span>
                  )}
                  <span className="strings-row-actions">
                    {isEditing ? (
                      <button
                        type="button"
                        className="strings-action-btn"
                        onMouseDown={(e) => e.preventDefault()}
                        onClick={handleCancelEdit}
                        title="Cancel"
                      >
                        X
                      </button>
                    ) : (
                      <button
                        type="button"
                        className="strings-action-btn strings-delete-btn"
                        onClick={() => handleDeleteEntry(entry)}
                        title="Delete"
                      >
                        Del
                      </button>
                    )}
                  </span>
                </div>
              );
            })}
          </div>

          {totalPages > 1 && (
            <div className="card-list-pagination">
              <button
                type="button"
                disabled={page <= 1}
                onClick={() => setPage((p) => Math.max(1, p - 1))}
              >
                Prev
              </button>
              <span>
                {page} / {totalPages}
              </span>
              <button
                type="button"
                disabled={page >= totalPages}
                onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
              >
                Next
              </button>
            </div>
          )}
        </>
      )}
    </>
  );
}
