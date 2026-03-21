import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { CommandInfo, Diagnostic, ResourceValue, ScriptInfo, VariableInfo } from '../ipc/types';

interface ConsoleEntry {
  text: string;
  level: 'info' | 'warn' | 'error';
}

const MAX_CONSOLE_LINES = 500;

export interface Tab {
  scriptId: string;
  name: string;
  content: string;
  savedContent: string;
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
  terminalOutput: ConsoleEntry[];
  terminalBusy: boolean;
  leftPanelOpen: boolean;
  bottomPanelOpen: boolean;
  rightPanelOpen: boolean;
  bottomPanelTab: string;
  isSimPaused: boolean;
  isDebugging: boolean;
  isPaused: boolean;
  debugLine: number | null;
  debugVariables: VariableInfo[];
  debugCallStack: string[];
  breakpoints: Record<string, number[]>;
  tier: number;
  foldedLines: Record<string, number[]>;
  availableCommands: string[];
  availableResources: string[];
  resourceValues: ResourceValue[];
  commandInfo: CommandInfo[];
  devMode: boolean;

  setTier: (tier: number) => void;
  openTab: (scriptId: string, name: string, content: string, scriptType: string) => void;
  closeTab: (scriptId: string) => void;
  switchTab: (scriptId: string) => void;
  updateContent: (scriptId: string, content: string) => void;
  markSaved: (scriptId: string) => void;
  setDiagnostics: (scriptId: string, diagnostics: Diagnostic[]) => void;
  setScriptList: (scripts: ScriptInfo[]) => void;
  setCursor: (line: number, col: number) => void;
  addConsoleOutput: (text: string, level: 'info' | 'warn' | 'error') => void;
  addConsoleOutputBatch: (entries: { text: string; level: 'info' | 'warn' | 'error' }[]) => void;
  clearConsole: () => void;
  addTerminalOutput: (text: string, level: 'info' | 'warn' | 'error') => void;
  addTerminalOutputBatch: (entries: { text: string; level: 'info' | 'warn' | 'error' }[]) => void;
  clearTerminal: () => void;
  setTerminalBusy: (busy: boolean) => void;
  toggleLeftPanel: () => void;
  toggleBottomPanel: () => void;
  toggleRightPanel: () => void;
  setBottomPanelOpen: (open: boolean) => void;
  setBottomPanelTab: (tab: string) => void;
  setSimPaused: (paused: boolean) => void;
  setDebugging: (debugging: boolean) => void;
  setPaused: (paused: boolean) => void;
  setDebugLine: (line: number | null) => void;
  setDebugVariables: (vars: VariableInfo[]) => void;
  setDebugCallStack: (stack: string[]) => void;
  toggleBreakpoint: (scriptId: string, line: number) => void;
  getBreakpoints: (scriptId: string) => number[];
  setFoldedLines: (scriptId: string, lines: number[]) => void;
  setAvailableCommands: (commands: string[]) => void;
  setAvailableResources: (resources: string[]) => void;
  setResourceValues: (values: ResourceValue[]) => void;
  setCommandInfo: (info: CommandInfo[]) => void;
  setDevMode: (devMode: boolean) => void;
}

export const useStore = create<EditorState>()(persist((set, get) => ({
  tabs: [],
  activeTabId: null,
  scriptList: [],
  cursorLine: 1,
  cursorCol: 1,
  consoleOutput: [],
  terminalOutput: [],
  terminalBusy: false,
  leftPanelOpen: true,
  bottomPanelOpen: true,
  rightPanelOpen: false,
  bottomPanelTab: 'terminal',
  isSimPaused: false,
  isDebugging: false,
  isPaused: false,
  debugLine: null,
  debugVariables: [],
  debugCallStack: [],
  tier: 0,
  breakpoints: {},
  foldedLines: {},
  availableCommands: [],
  availableResources: [],
  resourceValues: [],
  commandInfo: [],
  devMode: false,

  setTier: (tier) => set({ tier }),
  openTab: (scriptId, name, content, scriptType) =>
    set((state) => {
      const existing = state.tabs.find((t) => t.scriptId === scriptId);
      if (existing) {
        return { activeTabId: scriptId, cursorLine: 1, cursorCol: 1 };
      }
      return {
        tabs: [...state.tabs, { scriptId, name, content, savedContent: content, scriptType, isModified: false, diagnostics: [] }],
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
        t.scriptId === scriptId ? { ...t, content, isModified: content !== t.savedContent } : t
      ),
    })),

  markSaved: (scriptId) =>
    set((state) => ({
      tabs: state.tabs.map((t) =>
        t.scriptId === scriptId ? { ...t, savedContent: t.content, isModified: false } : t
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
    set((state) => {
      const next = state.consoleOutput.concat({ text, level });
      return { consoleOutput: next.length > MAX_CONSOLE_LINES ? next.slice(-MAX_CONSOLE_LINES) : next };
    }),

  addConsoleOutputBatch: (entries) =>
    set((state) => {
      if (entries.length === 0) return state;
      const next = state.consoleOutput.concat(entries);
      return { consoleOutput: next.length > MAX_CONSOLE_LINES ? next.slice(-MAX_CONSOLE_LINES) : next };
    }),

  clearConsole: () => set({ consoleOutput: [] }),

  addTerminalOutput: (text, level) =>
    set((state) => {
      const next = state.terminalOutput.concat({ text, level });
      return { terminalOutput: next.length > MAX_CONSOLE_LINES ? next.slice(-MAX_CONSOLE_LINES) : next };
    }),

  addTerminalOutputBatch: (entries) =>
    set((state) => {
      if (entries.length === 0) return state;
      const next = state.terminalOutput.concat(entries);
      return { terminalOutput: next.length > MAX_CONSOLE_LINES ? next.slice(-MAX_CONSOLE_LINES) : next };
    }),

  clearTerminal: () => set({ terminalOutput: [] }),

  setTerminalBusy: (busy) => set({ terminalBusy: busy }),

  toggleLeftPanel: () => set((state) => ({ leftPanelOpen: !state.leftPanelOpen })),
  toggleBottomPanel: () => set((state) => ({ bottomPanelOpen: !state.bottomPanelOpen })),
  toggleRightPanel: () => set((state) => ({ rightPanelOpen: !state.rightPanelOpen })),
  setBottomPanelOpen: (open) => set({ bottomPanelOpen: open }),
  setBottomPanelTab: (tab) => set({ bottomPanelTab: tab }),

  setSimPaused: (paused) => set({ isSimPaused: paused }),

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

  setFoldedLines: (key, lines) =>
    set((state) => ({
      foldedLines: { ...state.foldedLines, [key]: lines },
    })),

  setAvailableCommands: (commands) => set({ availableCommands: commands }),
  setAvailableResources: (resources) => set({ availableResources: resources }),
  setResourceValues: (values) => set({ resourceValues: values }),
  setCommandInfo: (info) => set({ commandInfo: info }),
  setDevMode: (devMode) => set({ devMode }),
}), {
  name: 'deadcode-editor-panels',
  partialize: (state) => ({
    tier: state.tier,
    leftPanelOpen: state.leftPanelOpen,
    bottomPanelOpen: state.bottomPanelOpen,
    rightPanelOpen: state.rightPanelOpen,
    bottomPanelTab: state.bottomPanelTab,
    foldedLines: state.foldedLines,
  }),
  onRehydrateStorage: () => (state) => {
    if (state && state.tier === 0) {
      useStore.setState({
        leftPanelOpen: false,
        rightPanelOpen: false,
        bottomPanelOpen: true,
      });
    }
  },
}));
