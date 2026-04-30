import { useEffect, useState, useCallback } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useShellStore } from "../shared/stores/shellStore";
import type { GlobalConfig } from "../shared/contracts/config";
import { useAppI18n } from "../shared/i18n";
import { PackMetadataPanel } from "../features/pack/PackMetadataPanel";
import { CardListPanel } from "../features/card/CardListPanel";
import { CardEditDrawer } from "../features/card/CardEditDrawer";
import { StringsListPanel } from "../features/strings/StringsListPanel";
import type { NoticeTone } from "./NoticeBanner";
import shared from "../shared/styles/shared.module.css";

type PackTab = "cards" | "strings";

interface PackWorkAreaProps {
  config: GlobalConfig;
  onNotice: (tone: NoticeTone, title: string, detail: string) => void;
  onPackDeleted: (packId: string) => void;
}

export function PackWorkArea({ config, onNotice, onPackDeleted }: PackWorkAreaProps) {
  const { td } = useAppI18n();
  const activePackId = useShellStore((s) => s.activePackId);
  const activeView = useShellStore((s) => s.activeView);
  const workspaceId = useShellStore((s) => s.workspaceId);
  const queryClient = useQueryClient();

  const [activeTab, setActiveTab] = useState<PackTab>("cards");
  const [editingCardId, setEditingCardId] = useState<string | null>(null);
  const [isCreatingCard, setIsCreatingCard] = useState(false);

  const activeCustomPackId =
    activeView?.type === "custom_pack" ? activeView.packId : activePackId;

  useEffect(() => {
    setEditingCardId(null);
    setIsCreatingCard(false);
    setActiveTab("cards");
  }, [activePackId, activeView]);

  const cardDrawerOpen = editingCardId !== null || isCreatingCard;

  const handleEditCard = useCallback((cardId: string) => {
    setEditingCardId(cardId);
    setIsCreatingCard(false);
  }, []);

  const handleNewCard = useCallback(() => {
    setEditingCardId(null);
    setIsCreatingCard(true);
  }, []);

  const handleDrawerClose = useCallback(() => {
    setEditingCardId(null);
    setIsCreatingCard(false);
  }, []);

  const handleDrawerSaved = useCallback(() => {
    void queryClient.invalidateQueries({ queryKey: ["cards"] });
  }, [queryClient]);

  if (!activeCustomPackId) return null;

  return (
    <>
      <PackMetadataPanel
        key={activeCustomPackId}
        config={config}
        onNotice={onNotice}
        onPackDeleted={onPackDeleted}
      >
        <div className={shared.tabStrip}>
          <button
            type="button"
            className={`${shared.tabBtn} ${activeTab === "cards" ? "active" : ""}`}
            onClick={() => setActiveTab("cards")}
          >
            {td("pack.tabs.cards", "Cards")}
          </button>
          <button
            type="button"
            className={`${shared.tabBtn} ${activeTab === "strings" ? "active" : ""}`}
            onClick={() => setActiveTab("strings")}
          >
            {td("pack.tabs.strings", "Strings")}
          </button>
        </div>

        <div className={shared.tabContent}>
          {activeTab === "cards" ? (
            <CardListPanel onEditCard={handleEditCard} onNewCard={handleNewCard} />
          ) : (
            <StringsListPanel catalog={config.text_language_catalog} />
          )}
        </div>
      </PackMetadataPanel>

      {cardDrawerOpen && workspaceId && (
        <CardEditDrawer
          packId={activeCustomPackId}
          workspaceId={workspaceId}
          cardId={isCreatingCard ? null : editingCardId}
          config={config}
          onClose={handleDrawerClose}
          onSaved={handleDrawerSaved}
        />
      )}
    </>
  );
}
