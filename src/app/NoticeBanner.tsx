import styles from "./NoticeBanner.module.css";

export type NoticeTone = "success" | "warning" | "error";

export interface Notice {
  tone: NoticeTone;
  title: string;
  detail: string;
}

interface NoticeBannerProps {
  notice: Notice;
  onDismiss: () => void;
}

export function NoticeBanner({ notice, onDismiss }: NoticeBannerProps) {
  return (
    <div className={styles.notice} data-tone={notice.tone}>
      <strong>{notice.title}</strong>
      <span>{notice.detail}</span>
      <button type="button" className={styles.noticeClose} onClick={onDismiss}>
        ×
      </button>
    </div>
  );
}
