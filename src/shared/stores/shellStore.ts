import { create } from "zustand";
import type { PackMetadata, PackOverview } from "../contracts/pack";
import type { ValidationIssue } from "../contracts/common";

export type ModalType =
  | "workspace"
  | "settings"
  | "export"
  | "addPack";

export type WorkspaceSubView = "recent" | "create" | "openByPath";
export type AddPackTab = "openPack" | "createPack" | "importPack";

export interface ModalState {
  type: ModalType;
  workspaceSubView?: WorkspaceSubView;
  addPackTab?: AddPackTab;
}

type DialogConfirmHandler = () => void | Promise<void>;

interface DialogBaseState {
  title: string;
  message: string;
  errorMessage?: string | null;
  confirmLabel: string;
  cancelLabel: string;
  onConfirm: DialogConfirmHandler;
}

export interface ConfirmDialogState {
  kind: "confirm";
  danger?: boolean;
}

export interface WarningDialogState {
  kind: "warning";
  warnings: ValidationIssue[];
}

export type DialogState =
  | (DialogBaseState & ConfirmDialogState)
  | (DialogBaseState & WarningDialogState);

interface ShellState {
  workspaceId: string | null;
  workspaceName: string | null;
  workspacePath: string | null;

  openPackIds: string[];
  activePackId: string | null;

  packMetadataMap: Record<string, PackMetadata>;
  packOverviews: PackOverview[];

  modal: ModalState | null;
  dialog: DialogState | null;
  dialogBusy: boolean;

  openModal: (type: ModalType) => void;
  closeModal: () => void;
  setWorkspaceSubView: (view: WorkspaceSubView) => void;
  setAddPackTab: (tab: AddPackTab) => void;
  openDialog: (dialog: DialogState) => void;
  closeDialog: () => void;
  setDialogBusy: (busy: boolean) => void;
  setDialogError: (message: string | null) => void;

  setWorkspace: (id: string, name: string, path: string) => void;
  clearWorkspace: () => void;

  setPackOverviews: (overviews: PackOverview[]) => void;

  setOpenPacks: (ids: string[], activeId: string | null) => void;
  setActivePack: (id: string | null) => void;
  addOpenPack: (id: string, metadata: PackMetadata) => void;
  updatePackMetadata: (id: string, metadata: PackMetadata) => void;
  removeOpenPack: (id: string) => void;
}

export const useShellStore = create<ShellState>()((set) => ({
  workspaceId: null,
  workspaceName: null,
  workspacePath: null,

  openPackIds: [],
  activePackId: null,

  packMetadataMap: {},
  packOverviews: [],

  modal: null,
  dialog: null,
  dialogBusy: false,

  openModal: (type) =>
    set({
      modal: {
        type,
        workspaceSubView: type === "workspace" ? "recent" : undefined,
        addPackTab: type === "addPack" ? "openPack" : undefined,
      },
    }),

  closeModal: () => set({ modal: null }),
  openDialog: (dialog) => set({ dialog: { ...dialog, errorMessage: null }, dialogBusy: false }),
  closeDialog: () => set({ dialog: null, dialogBusy: false }),
  setDialogBusy: (busy) => set({ dialogBusy: busy }),
  setDialogError: (message) =>
    set((state) => {
      if (!state.dialog) return state;
      return {
        dialog: {
          ...state.dialog,
          errorMessage: message,
        },
      };
    }),

  setWorkspaceSubView: (view) =>
    set((state) => {
      if (!state.modal || state.modal.type !== "workspace") return state;
      return { modal: { ...state.modal, workspaceSubView: view } };
    }),

  setAddPackTab: (tab) =>
    set((state) => {
      if (!state.modal || state.modal.type !== "addPack") return state;
      return { modal: { ...state.modal, addPackTab: tab } };
    }),

  setWorkspace: (id, name, path) =>
    set({
      workspaceId: id,
      workspaceName: name,
      workspacePath: path,
      openPackIds: [],
      activePackId: null,
      packMetadataMap: {},
      packOverviews: [],
      dialog: null,
      dialogBusy: false,
    }),

  clearWorkspace: () =>
    set({
      workspaceId: null,
      workspaceName: null,
      workspacePath: null,
      openPackIds: [],
      activePackId: null,
      packMetadataMap: {},
      packOverviews: [],
      dialog: null,
      dialogBusy: false,
    }),

  setPackOverviews: (overviews) => set({ packOverviews: overviews }),

  setOpenPacks: (ids, activeId) =>
    set({ openPackIds: ids, activePackId: activeId }),

  setActivePack: (id) => set({ activePackId: id }),

  addOpenPack: (id, metadata) =>
    set((state) => {
      const nextMap = { ...state.packMetadataMap, [id]: metadata };
      if (state.openPackIds.includes(id)) {
        return { activePackId: id, packMetadataMap: nextMap };
      }
      return {
        openPackIds: [...state.openPackIds, id],
        activePackId: id,
        packMetadataMap: nextMap,
      };
    }),

  updatePackMetadata: (id, metadata) =>
    set((state) => ({
      packMetadataMap: { ...state.packMetadataMap, [id]: metadata },
    })),

  removeOpenPack: (id) =>
    set((state) => {
      const ids = state.openPackIds.filter((pid) => pid !== id);
      const activeId =
        state.activePackId === id
          ? ids[ids.length - 1] ?? null
          : state.activePackId;
      const { [id]: _, ...nextMap } = state.packMetadataMap;
      return { openPackIds: ids, activePackId: activeId, packMetadataMap: nextMap };
    }),
}));
