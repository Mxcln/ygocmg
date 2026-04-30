import { useEffect } from "react";
import { useAppI18n } from "../shared/i18n";
import styles from "./NoticeBanner.module.css";

export type NoticeTone = "success" | "warning" | "error";

export interface Notice {
  id: number;
  tone: NoticeTone;
  title: string;
  detail: string;
}

interface NoticeBannerProps {
  notices: Notice[];
  onDismiss: (id: number) => void;
}

const NOTICE_TIMEOUT_MS: Record<NoticeTone, number> = {
  success: 3800,
  warning: 5600,
  error: 7000,
};

export function NoticeBanner({ notices, onDismiss }: NoticeBannerProps) {
  if (notices.length === 0) return null;

  return (
    <div className={styles.noticeStack} aria-live="polite" aria-atomic="false">
      {notices.map((notice) => (
        <NoticeToast key={notice.id} notice={notice} onDismiss={onDismiss} />
      ))}
    </div>
  );
}

function NoticeToast({
  notice,
  onDismiss,
}: {
  notice: Notice;
  onDismiss: (id: number) => void;
}) {
  const { t } = useAppI18n();

  useEffect(() => {
    const timer = window.setTimeout(() => {
      onDismiss(notice.id);
    }, NOTICE_TIMEOUT_MS[notice.tone]);

    return () => window.clearTimeout(timer);
  }, [notice.id, notice.tone, onDismiss]);

  return (
    <div
      className={styles.notice}
      data-tone={notice.tone}
      role={notice.tone === "error" ? "alert" : "status"}
    >
      <div className={styles.noticeBody}>
        <strong>{notice.title}</strong>
        <span>{notice.detail}</span>
      </div>
      <button
        type="button"
        className={styles.noticeClose}
        onClick={() => onDismiss(notice.id)}
        aria-label={t("notice.dismiss")}
      >
        ×
      </button>
    </div>
  );
}
