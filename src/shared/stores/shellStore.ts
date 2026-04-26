import { create } from "zustand";

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

interface ShellState {
  workspaceId: string | null;
  workspaceName: string | null;

  openPackIds: string[];
  activePackId: string | null;

  modal: ModalState | null;

  openModal: (type: ModalType) => void;
  closeModal: () => void;
  setWorkspaceSubView: (view: WorkspaceSubView) => void;
  setAddPackTab: (tab: AddPackTab) => void;

  setWorkspace: (id: string, name: string) => void;
  clearWorkspace: () => void;

  setOpenPacks: (ids: string[], activeId: string | null) => void;
  setActivePack: (id: string | null) => void;
  addOpenPack: (id: string) => void;
  removeOpenPack: (id: string) => void;
}

export const useShellStore = create<ShellState>()((set) => ({
  workspaceId: null,
  workspaceName: null,

  openPackIds: [],
  activePackId: null,

  modal: null,

  openModal: (type) =>
    set({
      modal: {
        type,
        workspaceSubView: type === "workspace" ? "recent" : undefined,
        addPackTab: type === "addPack" ? "openPack" : undefined,
      },
    }),

  closeModal: () => set({ modal: null }),

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

  setWorkspace: (id, name) =>
    set({ workspaceId: id, workspaceName: name, openPackIds: [], activePackId: null }),

  clearWorkspace: () =>
    set({ workspaceId: null, workspaceName: null, openPackIds: [], activePackId: null }),

  setOpenPacks: (ids, activeId) =>
    set({ openPackIds: ids, activePackId: activeId }),

  setActivePack: (id) => set({ activePackId: id }),

  addOpenPack: (id) =>
    set((state) => {
      if (state.openPackIds.includes(id)) return { activePackId: id };
      return { openPackIds: [...state.openPackIds, id], activePackId: id };
    }),

  removeOpenPack: (id) =>
    set((state) => {
      const ids = state.openPackIds.filter((pid) => pid !== id);
      const activeId =
        state.activePackId === id
          ? ids[ids.length - 1] ?? null
          : state.activePackId;
      return { openPackIds: ids, activePackId: activeId };
    }),
}));
