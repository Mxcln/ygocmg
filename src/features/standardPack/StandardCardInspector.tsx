import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { convertFileSrc } from "@tauri-apps/api/core";
import { standardPackApi } from "../../shared/api/standardPackApi";
import type { CardEntity } from "../../shared/contracts/card";
import { CardInfoForm } from "../card/CardInfoForm";
import { CardTextForm } from "../card/CardTextForm";

interface StandardCardInspectorProps {
  code: number;
  onClose: () => void;
}

type InspectorTab = "text" | "info";

function textLanguages(card: CardEntity, available: string[]): string[] {
  return available.length > 0 ? available : Object.keys(card.texts);
}

export function StandardCardInspector({ code, onClose }: StandardCardInspectorProps) {
  const [activeTab, setActiveTab] = useState<InspectorTab>("text");

  const { data: detail, isLoading, error } = useQuery({
    queryKey: ["standard-card", code],
    queryFn: () => standardPackApi.getCard({ code }),
  });

  const card = detail?.card ?? null;
  const languages = card ? textLanguages(card, detail?.available_languages ?? []) : [];
  const titleLanguage = languages[0] ?? "";
  const imageSrc = detail?.asset_state.has_image && detail.ygopro_path
    ? `${convertFileSrc(`${detail.ygopro_path}/pics/${code}.jpg`)}?standard=${code}`
    : null;

  return (
    <>
      <div className="card-edit-backdrop" onClick={onClose} />
      <div className="card-edit-drawer standard-inspector">
        <div className="card-edit-header">
          <div className="card-edit-header-left">
            <button type="button" className="ghost-button" onClick={onClose}>
              Close
            </button>
            <div className="standard-inspector-title">
              <strong>{card?.texts[titleLanguage]?.name || `Card ${code}`}</strong>
              <span>{code}</span>
            </div>
          </div>
          <div className="card-edit-header-spacer" />
          <span className="readonly-chip">Read-only</span>
        </div>

        {error && (
          <div className="card-edit-error">Failed to load standard card.</div>
        )}

        {isLoading && !card ? (
          <div className="card-list-empty">
            <p>Loading card...</p>
          </div>
        ) : card && detail ? (
          <div className="card-edit-body">
            <div className="card-asset-bar standard-asset-bar">
              <div className="card-pic-preview">
                {imageSrc ? <img src={imageSrc} alt="Card" /> : "No Image"}
              </div>
              <div className="asset-readonly-grid">
                <span>Image</span>
                <strong>{detail.asset_state.has_image ? "Present" : "Missing"}</strong>
                <span>Script</span>
                <strong>{detail.asset_state.has_script ? "Present" : "Missing"}</strong>
                <span>Field</span>
                <strong>{detail.asset_state.has_field_image ? "Present" : "Missing"}</strong>
              </div>
            </div>

            <div className="card-form-area">
              <div className="card-form-tabs">
                <button
                  type="button"
                  className={`card-form-tab ${activeTab === "text" ? "active" : ""}`}
                  onClick={() => setActiveTab("text")}
                >
                  Text
                </button>
                <button
                  type="button"
                  className={`card-form-tab ${activeTab === "info" ? "active" : ""}`}
                  onClick={() => setActiveTab("info")}
                >
                  Info
                </button>
              </div>

              <div className="card-form-content">
                {activeTab === "text" ? (
                  <CardTextForm
                    draft={card}
                    availableLanguages={languages}
                    displayLanguageOrder={languages}
                    readonly
                  />
                ) : (
                  <CardInfoForm draft={card} readonly />
                )}
              </div>
            </div>
          </div>
        ) : null}
      </div>
    </>
  );
}
