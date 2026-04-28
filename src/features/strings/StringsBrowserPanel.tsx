import { useEffect, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { formatStringKeyHex, parseHexInput } from "../../shared/utils/format";
import type { PackStringEntry, PackStringKind, PackStringsPage } from "../../shared/contracts/strings";

const PAGE_SIZE = 50;

const KIND_OPTIONS: { value: PackStringKind | ""; label: string }[] = [
  { value: "", label: "All Kinds" },
  { value: "system", label: "System" },
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

export interface StringsBrowserQuery {
  language: string;
  kindFilter: PackStringKind | "";
  keyFilter: number | null;
  keyword: string;
  page: number;
  pageSize: number;
}

interface StringsBrowserPanelProps {
  enabled: boolean;
  queryKeyBase: readonly unknown[];
  languages: string[];
  loadPage: (query: StringsBrowserQuery) => Promise<PackStringsPage>;
  editable?: boolean;
  emptyTitle: string;
  emptyHint: string;
  errorMessage?: string | null;
  saving?: boolean;
  onCreate?: (entry: PackStringEntry, language: string) => Promise<void>;
  onUpdate?: (entry: PackStringEntry, language: string) => Promise<void>;
  onDelete?: (entry: PackStringEntry) => void;
}

function normalizeHexDraft(value: string): string {
  const trimmed = value.trim();
  const withoutPrefix = trimmed.replace(/^0x/i, "");
  return withoutPrefix.replace(/[^0-9a-fA-F]/g, "").toUpperCase();
}

export function StringsBrowserPanel({
  enabled,
  queryKeyBase,
  languages,
  loadPage,
  editable = false,
  emptyTitle,
  emptyHint,
  errorMessage = null,
  saving = false,
  onCreate,
  onUpdate,
  onDelete,
}: StringsBrowserPanelProps) {
  const [language, setLanguage] = useState(languages[0] ?? "");
  const [kindFilter, setKindFilter] = useState<PackStringKind | "">("");
  const [keyword, setKeyword] = useState("");
  const [page, setPage] = useState(1);
  const [editingCell, setEditingCell] = useState<EditingCell | null>(null);
  const [newRow, setNewRow] = useState<NewRow | null>(null);
  const [localError, setLocalError] = useState<string | null>(null);
  const editInputRef = useRef<HTMLInputElement>(null);
  const newKeyRef = useRef<HTMLInputElement>(null);
  const previousHadNewRow = useRef(false);

  useEffect(() => {
    if (languages.length > 0 && !languages.includes(language)) {
      setLanguage(languages[0]);
    }
  }, [languages, language]);

  useEffect(() => {
    if (editInputRef.current) editInputRef.current.focus();
  }, [editingCell]);

  useEffect(() => {
    if (newRow && !previousHadNewRow.current && newKeyRef.current) {
      newKeyRef.current.focus();
    }
    previousHadNewRow.current = newRow !== null;
  }, [newRow]);

  const canLoad = enabled && !!language;
  const { data, isLoading, error: queryError } = useQuery<PackStringsPage>({
    queryKey: [...queryKeyBase, language, kindFilter, keyword, page],
    queryFn: () =>
      loadPage({
        language,
        kindFilter,
        keyFilter: null,
        keyword,
        page,
        pageSize: PAGE_SIZE,
      }),
    enabled: canLoad,
  });

  const items = data?.items ?? [];
  const total = data?.total ?? 0;
  const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE));
  const shownError = errorMessage ?? localError;

  function handleStartEdit(entry: PackStringEntry) {
    if (!editable || saving) return;
    setEditingCell({ kind: entry.kind, key: entry.key, value: entry.value });
    setNewRow(null);
    setLocalError(null);
  }

  async function handleCommitEdit() {
    if (!editable || !editingCell || saving || !onUpdate) return;
    const original = items.find(
      (entry) => entry.kind === editingCell.kind && entry.key === editingCell.key,
    );
    if (original && original.value === editingCell.value) {
      setEditingCell(null);
      return;
    }
    await onUpdate(
      {
        kind: editingCell.kind,
        key: editingCell.key,
        value: editingCell.value,
      },
      language,
    );
    setEditingCell(null);
  }

  function handleEditKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      e.stopPropagation();
      void handleCommitEdit();
    } else if (e.key === "Escape") {
      e.stopPropagation();
      setEditingCell(null);
    }
  }

  async function handleCommitNew() {
    if (!editable || !newRow || saving || !onCreate) return;
    const parsedKey = parseHexInput(newRow.key);
    if (isNaN(parsedKey) || parsedKey < 0) {
      setLocalError("Key must be a non-negative hexadecimal value.");
      return;
    }
    if (!newRow.value.trim()) {
      setLocalError("Value cannot be empty.");
      return;
    }
    await onCreate({ kind: newRow.kind, key: parsedKey, value: newRow.value }, language);
    setNewRow(null);
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

  if (!language && languages.length === 0) {
    return (
      <div className="card-list-empty">
        <p>No languages available.</p>
      </div>
    );
  }

  return (
    <>
      <div className="strings-toolbar">
        <select
          className="strings-lang-select"
          value={language}
          disabled={languages.length <= 1}
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
        {editable && (
          <button
            type="button"
            className="primary-button"
            onClick={() => {
              setNewRow({ kind: "counter", key: "", value: "" });
              setEditingCell(null);
              setLocalError(null);
            }}
          >
            + New String
          </button>
        )}
      </div>

      {shownError && <div className="strings-error">{shownError}</div>}

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
          <p>{emptyTitle}</p>
          <p>{emptyHint}</p>
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
                  <option value="system">System</option>
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
                      title={editable ? "Double-click to edit" : undefined}
                    >
                      {entry.value || "\u00A0"}
                    </span>
                  )}
                  <span className="strings-row-actions">
                    {editable && isEditing ? (
                      <button
                        type="button"
                        className="strings-action-btn"
                        onMouseDown={(e) => e.preventDefault()}
                        onClick={() => setEditingCell(null)}
                        title="Cancel"
                      >
                        X
                      </button>
                    ) : editable ? (
                      <button
                        type="button"
                        className="strings-action-btn strings-delete-btn"
                        onClick={() => onDelete?.(entry)}
                        title="Delete"
                      >
                        Del
                      </button>
                    ) : null}
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
