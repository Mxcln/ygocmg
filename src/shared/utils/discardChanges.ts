import type { DialogState } from "../stores/shellStore";

interface PromptDiscardChangesOptions {
  title: string;
  message: string;
  confirmLabel: string;
  cancelLabel: string;
  openDialog: (dialog: DialogState) => void;
  closeDialog: () => void;
  onDiscard: () => void | Promise<void>;
}

export function promptDiscardChanges({
  title,
  message,
  confirmLabel,
  cancelLabel,
  openDialog,
  closeDialog,
  onDiscard,
}: PromptDiscardChangesOptions) {
  openDialog({
    kind: "confirm",
    title,
    message,
    confirmLabel,
    cancelLabel,
    danger: true,
    onConfirm: async () => {
      closeDialog();
      await onDiscard();
    },
  });
}
