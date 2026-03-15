import { create } from 'zustand';
import type { Diagnostic, ScriptInfo, VariableInfo } from '../ipc/types';

interface ConsoleEntry {
  text: string;
  level: 'info' | 'warn' | 'error';
}

export interface Tab {
  scriptId: string;
  name: string;
  content: string;
  scriptType: string;
  isModified: boolean;
  diagnostics: Diagnostic[];
}

interface EditorState {
  tabs: Tab[];
  activeTabId: string | null;
  scriptList: ScriptInfo[];
  cursorLine: number;
  cursorCol: number;
  consoleOutput: ConsoleEntry[];
  leftPanelOpen: boolean;
  bottomPanelOpen: boolean;
  rightPanelOpen: boolean;
  bottomPanelTab: string;
  isRunning: boolean;
  isDebugging: boolean;
  isPaused: boolean;
  debugLine: number | null;
  debugVariables: VariableInfo[];
  debugCallStack: string[];
  breakpoints: Record<string, number[]>;

  openTab: (scriptId: string, name: string, content: string, scriptType: string) => void;
  closeTab: (scriptId: string) => void;
  switchTab: (scriptId: string) => void;
  updateContent: (scriptId: string, content: string) => void;
  setDiagnostics: (scriptId: string, diagnostics: Diagnostic[]) => void;
  setScriptList: (scripts: ScriptInfo[]) => void;
  setCursor: (line: number, col: number) => void;
  addConsoleOutput: (text: string, level: 'info' | 'warn' | 'error') => void;
  clearConsole: () => void;
  toggleLeftPanel: () => void;
  toggleBottomPanel: () => void;
  toggleRightPanel: () => void;
  setBottomPanelOpen: (open: boolean) => void;
  setBottomPanelTab: (tab: string) => void;
  setRunning: (running: boolean) => void;
  setDebugging: (debugging: boolean) => void;
  setPaused: (paused: boolean) => void;
  setDebugLine: (line: number | null) => void;
  setDebugVariables: (vars: VariableInfo[]) => void;
  setDebugCallStack: (stack: string[]) => void;
  toggleBreakpoint: (scriptId: string, line: number) => void;
  getBreakpoints: (scriptId: string) => number[];
}

export const useStore = create<EditorState>((set, get) => ({
  tabs: [],
  activeTabId: null,
  scriptList: [],
  cursorLine: 1,
  cursorCol: 1,
  consoleOutput: [],
  leftPanelOpen: true,
  bottomPanelOpen: false,
  rightPanelOpen: false,
  bottomPanelTab: 'console',
  isRunning: false,
  isDebugging: false,
  isPaused: false,
  debugLine: null,
  debugVariables: [],
  debugCallStack: [],
  breakpoints: {},

  openTab: (scriptId, name, content, scriptType) =>
    set((state) => {
      const existing = state.tabs.find((t) => t.scriptId === scriptId);
      if (existing) {
        return { activeTabId: scriptId, cursorLine: 1, cursorCol: 1 };
      }
      return {
        tabs: [...state.tabs, { scriptId, name, content, scriptType, isModified: false, diagnostics: [] }],
        activeTabId: scriptId,
        cursorLine: 1,
        cursorCol: 1,
      };
    }),

  closeTab: (scriptId) =>
    set((state) => {
      const newTabs = state.tabs.filter((t) => t.scriptId !== scriptId);
      let newActiveId = state.activeTabId;
      if (state.activeTabId === scriptId) {
        const idx = state.tabs.findIndex((t) => t.scriptId === scriptId);
        newActiveId = newTabs.length > 0
          ? newTabs[Math.min(idx, newTabs.length - 1)].scriptId
          : null;
      }
      return { tabs: newTabs, activeTabId: newActiveId };
    }),

  switchTab: (scriptId) => set({ activeTabId: scriptId, cursorLine: 1, cursorCol: 1 }),

  updateContent: (scriptId, content) =>
    set((state) => ({
      tabs: state.tabs.map((t) =>
        t.scriptId === scriptId ? { ...t, content, isModified: true } : t
      ),
    })),

  setDiagnostics: (scriptId, diagnostics) =>
    set((state) => ({
      tabs: state.tabs.map((t) =>
        t.scriptId === scriptId ? { ...t, diagnostics } : t
      ),
    })),

  setScriptList: (scripts) => set({ scriptList: scripts }),

  setCursor: (line, col) => set({ cursorLine: line, cursorCol: col }),

  addConsoleOutput: (text, level) =>
    set((state) => ({
      consoleOutput: [...state.consoleOutput, { text, level }],
    })),

  clearConsole: () => set({ consoleOutput: [] }),

  toggleLeftPanel: () => set((state) => ({ leftPanelOpen: !state.leftPanelOpen })),
  toggleBottomPanel: () => set((state) => ({ bottomPanelOpen: !state.bottomPanelOpen })),
  toggleRightPanel: () => set((state) => ({ rightPanelOpen: !state.rightPanelOpen })),
  setBottomPanelOpen: (open) => set({ bottomPanelOpen: open }),
  setBottomPanelTab: (tab) => set({ bottomPanelTab: tab }),

  setRunning: (running) => set({ isRunning: running }),

  setDebugging: (debugging) => set((state) => ({
    isDebugging: debugging,
    ...(debugging ? { rightPanelOpen: true, bottomPanelOpen: true } : {}),
  })),

  setPaused: (paused) => set({ isPaused: paused }),

  setDebugLine: (line) => set({ debugLine: line }),

  setDebugVariables: (vars) => set({ debugVariables: vars }),

  setDebugCallStack: (stack) => set({ debugCallStack: stack }),

  toggleBreakpoint: (scriptId, line) =>
    set((state) => {
      const current = state.breakpoints[scriptId] || [];
      const newBps = current.includes(line)
        ? current.filter((l) => l !== line)
        : [...current, line];
      return { breakpoints: { ...state.breakpoints, [scriptId]: newBps } };
    }),

  getBreakpoints: (scriptId) => get().breakpoints[scriptId] || [],
}));
