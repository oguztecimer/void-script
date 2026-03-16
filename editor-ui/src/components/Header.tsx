import styles from './Header.module.css';
import { useStore } from '../state/store';
import { sendToRust } from '../ipc/bridge';
import { ToolBtn } from '../primitives/ToolBtn';
import { Separator } from '../primitives/Separator';
import { Tooltip } from '../primitives/Tooltip';

const isWindows = navigator.platform.startsWith('Win');

function handleDragStart(e: React.MouseEvent) {
  // Only drag if not clicking an interactive element
  if ((e.target as HTMLElement).closest('.titlebar-no-drag')) return;
  sendToRust({ type: 'window_drag_start' });
}

export function Header() {
  const activeTabId = useStore((s) => s.activeTabId);
  const tabs = useStore((s) => s.tabs);
  const activeTab = tabs.find((t) => t.scriptId === activeTabId);
  const isRunning = useStore((s) => s.isRunning);
  const isDebugging = useStore((s) => s.isDebugging);
  const isPaused = useStore((s) => s.isPaused);
  const active = isRunning || isDebugging;

  return (
    <div className={styles.toolbar} onMouseDown={handleDragStart}>
      {/* Space for native macOS traffic lights (hidden on Windows) */}
      {!isWindows && <div className={styles.trafficLightSpacer} />}

      {/* Hamburger menu */}
      <ToolBtn size="small" title="Menu" className="titlebar-no-drag">
        <svg width="20" height="20" viewBox="0 0 16 16">
          <line x1="2" y1="4" x2="14" y2="4" stroke="currentColor" strokeWidth="1"/>
          <line x1="2" y1="8" x2="14" y2="8" stroke="currentColor" strokeWidth="1"/>
          <line x1="2" y1="12" x2="14" y2="12" stroke="currentColor" strokeWidth="1"/>
        </svg>
      </ToolBtn>

      <span className={styles.brandName}>
        <span className={styles.brandPunc}>[</span>
        <span className={styles.brandDead}>DEAD</span>
        <span className={styles.brandPunc}>//</span>
        <span className={styles.brandCode}>CODE</span>
        <span className={styles.brandPunc}>]</span>
      </span>

      {/* Center spacer - draggable */}
      <div className={styles.spacer} />

      {/* Right: run config + run/debug + search + settings */}
      <div className={`titlebar-no-drag ${styles.rightGroup}`}>
        {/* Run configuration selector */}
        <RunConfigSelector label={activeTab ? `${activeTab.name}.vs` : 'No configuration'} />

        <Separator variant="line" level="subtle" />

        {/* Run/Debug/Stop buttons */}
        {!active ? (
          <>
            <ToolBtn
              size="small"
              variant="filled"
              title="Run"
              shortcut="Shift+F10"
              bgColor="var(--bg-btn-run)"
              hoverBgColor="var(--bg-btn-run-hover)"
              iconColor="var(--icon-run)"
              onClick={() => activeTabId && sendToRust({ type: 'run_script', script_id: activeTabId })}
              disabled={!activeTabId}
            >
              <svg width="20" height="20" viewBox="0 0 16 16"><path d="M4 2l10 6-10 6V2z" fill="currentColor"/></svg>
            </ToolBtn>
            <ToolBtn
              size="small"
              variant="filled"
              title="Debug"
              shortcut="Shift+F9"
              bgColor="var(--bg-btn-debug)"
              hoverBgColor="var(--bg-btn-debug-hover)"
              iconColor="var(--icon-debug)"
              onClick={() => activeTabId && sendToRust({ type: 'debug_start', script_id: activeTabId })}
              disabled={!activeTabId}
            >
              <svg width="20" height="20" viewBox="0 0 16 16">
                <circle cx="8" cy="9" r="5" stroke="currentColor" strokeWidth="1.5" fill="none"/>
                <line x1="8" y1="4" x2="6" y2="1" stroke="currentColor" strokeWidth="1.5"/>
                <line x1="8" y1="4" x2="10" y2="1" stroke="currentColor" strokeWidth="1.5"/>
                <line x1="3" y1="7" x2="13" y2="7" stroke="currentColor" strokeWidth="1"/>
                <line x1="3" y1="10" x2="13" y2="10" stroke="currentColor" strokeWidth="1"/>
              </svg>
            </ToolBtn>
          </>
        ) : (
          <>
            <ToolBtn
              size="small"
              variant="filled"
              title="Stop"
              shortcut="Ctrl+F2"
              bgColor="var(--bg-btn-stop)"
              hoverBgColor="var(--bg-btn-stop-hover)"
              iconColor="var(--icon-stop)"
              onClick={() => activeTabId && sendToRust({ type: 'stop_script', script_id: activeTabId })}
            >
              <svg width="14" height="14" viewBox="0 0 10 10"><rect width="10" height="10" rx="1.5" fill="currentColor"/></svg>
            </ToolBtn>
            {isDebugging && isPaused && (
              <>
                <Separator variant="line" level="subtle" />
                <ToolBtn
                  size="small"
                  variant="filled"
                  title="Resume"
                  shortcut="F9"
                  bgColor="var(--bg-btn-run)"
                  hoverBgColor="var(--bg-btn-run-hover)"
                  iconColor="var(--icon-run)"
                  onClick={() => activeTabId && sendToRust({ type: 'debug_continue', script_id: activeTabId })}
                >
                  <svg width="20" height="20" viewBox="0 0 16 16"><path d="M4 2l10 6-10 6V2z" fill="currentColor"/></svg>
                </ToolBtn>
                <ToolBtn size="small" title="Step Over" shortcut="F8" onClick={() => activeTabId && sendToRust({ type: 'debug_step_over', script_id: activeTabId })}>
                  <svg width="20" height="20" viewBox="0 0 16 16"><path d="M2 12h4V8h4v4h4L8 4z" fill="currentColor" transform="rotate(90 8 8)"/></svg>
                </ToolBtn>
                <ToolBtn size="small" title="Step Into" shortcut="F7" onClick={() => activeTabId && sendToRust({ type: 'debug_step_into', script_id: activeTabId })}>
                  <svg width="20" height="20" viewBox="0 0 16 16"><path d="M8 2v8m-3-3l3 3 3-3M5 14h6" stroke="currentColor" strokeWidth="1.5" fill="none"/></svg>
                </ToolBtn>
                <ToolBtn size="small" title="Step Out" shortcut="Shift+F8" onClick={() => activeTabId && sendToRust({ type: 'debug_step_out', script_id: activeTabId })}>
                  <svg width="20" height="20" viewBox="0 0 16 16"><path d="M8 14V6m-3 3l3-3 3 3M5 2h6" stroke="currentColor" strokeWidth="1.5" fill="none"/></svg>
                </ToolBtn>
              </>
            )}
          </>
        )}

        <Separator variant="line" level="subtle" />
        <SearchPill />
        <ToolBtn size="small" title="Settings">
          <svg width="20" height="20" viewBox="0 0 16 16">
            <path d="M6.5.5h3l.5 2.1a5.5 5.5 0 0 1 1.3.8l2-.8 1.5 2.6-1.5 1.3a5.5 5.5 0 0 1 0 1.5l1.5 1.3-1.5 2.6-2-.8a5.5 5.5 0 0 1-1.3.8l-.5 2.1h-3l-.5-2.1a5.5 5.5 0 0 1-1.3-.8l-2 .8L1.2 9.3l1.5-1.3a5.5 5.5 0 0 1 0-1.5L1.2 5.2l1.5-2.6 2 .8A5.5 5.5 0 0 1 6 2.6L6.5.5z" stroke="currentColor" strokeWidth="1" fill="none"/>
            <circle cx="8" cy="8" r="2" stroke="currentColor" strokeWidth="1" fill="none"/>
          </svg>
        </ToolBtn>
      </div>

      {/* Windows window controls (minimize/maximize/close) */}
      {isWindows && <WindowControlsWin />}
    </div>
  );
}

/* --- Window Controls (Windows) --- */

function WindowControlsWin() {
  return (
    <div className={`titlebar-no-drag ${styles.winControls}`}>
      <button
        className={styles.winBtn}
        onClick={() => sendToRust({ type: 'window_minimize' })}
        title="Minimize"
      >
        <svg width="12" height="12" viewBox="0 0 12 12">
          <line x1="1" y1="6" x2="11" y2="6" stroke="currentColor" strokeWidth="1" />
        </svg>
      </button>
      <button
        className={styles.winBtn}
        onClick={() => sendToRust({ type: 'window_maximize' })}
        title="Maximize"
      >
        <svg width="12" height="12" viewBox="0 0 12 12">
          <rect x="1" y="1" width="10" height="10" stroke="currentColor" strokeWidth="1" fill="none" />
        </svg>
      </button>
      <button
        className={`${styles.winBtn} ${styles.winBtnClose}`}
        onClick={() => sendToRust({ type: 'window_close' })}
        title="Close"
      >
        <svg width="12" height="12" viewBox="0 0 12 12">
          <line x1="1" y1="1" x2="11" y2="11" stroke="currentColor" strokeWidth="1.2" />
          <line x1="11" y1="1" x2="1" y2="11" stroke="currentColor" strokeWidth="1.2" />
        </svg>
      </button>
    </div>
  );
}

/* --- Window Controls (macOS traffic lights) --- */

function WindowControls() {
  return (
    <div className={`titlebar-no-drag ${styles.trafficLights}`}>
      <TrafficLight
        color="var(--traffic-close)"
        hoverSymbol="×"
        onClick={() => sendToRust({ type: 'window_close' })}
        title="Close"
      />
      <TrafficLight
        color="var(--traffic-minimize)"
        hoverSymbol="−"
        onClick={() => sendToRust({ type: 'window_minimize' })}
        title="Minimize"
      />
      <TrafficLight
        color="var(--traffic-maximize)"
        hoverSymbol="+"
        onClick={() => sendToRust({ type: 'window_maximize' })}
        title="Maximize"
      />
    </div>
  );
}

function TrafficLight({ color, hoverSymbol, onClick, title }: {
  color: string;
  hoverSymbol: string;
  onClick: () => void;
  title: string;
}) {
  return (
    <Tooltip content={title}>
      <div
        onClick={onClick}
        className={styles.trafficLight}
        style={{ backgroundColor: color }}
      >
        {hoverSymbol}
      </div>
    </Tooltip>
  );
}

/* --- Header widget (project, VCS) — in drag zone, hover is JS-driven --- */

function HeaderWidget({ icon, label, muted, hasDropdown }: {
  icon: React.ReactNode;
  label: string;
  muted?: boolean;
  hasDropdown?: boolean;
}) {
  return (
    <button
      className={`titlebar-no-drag ${styles.widget} ${muted ? styles.widgetMuted : ''}`}
      onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = 'var(--bg-hover)'; }}
      onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = ''; }}
    >
      <span className={styles.widgetIcon}>{icon}</span>
      <span>{label}</span>
      {hasDropdown && (
        <svg width="8" height="8" viewBox="0 0 8 8" className={styles.widgetChevron}>
          <path d="M1 2.5l3 3 3-3" stroke="currentColor" strokeWidth="1.2" fill="none"/>
        </svg>
      )}
    </button>
  );
}

/* --- Run configuration selector --- */

function RunConfigSelector({ label }: { label: string }) {
  return (
    <button className={styles.runConfig}>
      <span className={styles.runConfigIcon}>
        <svg width="12" height="12" viewBox="0 0 16 16">
          <path d="M4 2l10 6-10 6V2z" fill="currentColor"/>
        </svg>
      </span>
      <span>{label}</span>
      <span className={styles.runConfigChevron}>
        <svg width="8" height="8" viewBox="0 0 8 8">
          <path d="M1 2.5l3 3 3-3" stroke="currentColor" strokeWidth="1.2" fill="none"/>
        </svg>
      </span>
    </button>
  );
}

/* --- Search Everywhere pill --- */

function SearchPill() {
  return (
    <Tooltip content="Search Everywhere (Shift Shift)">
      <button className={styles.searchPill}>
        <svg width="16" height="16" viewBox="0 0 16 16">
          <circle cx="6.5" cy="6.5" r="4.5" stroke="currentColor" strokeWidth="1.5" fill="none"/>
          <line x1="10" y1="10" x2="14" y2="14" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/>
        </svg>
        <span>Search</span>
        <span className={styles.searchShortcut}>&#8679;&#8679;</span>
      </button>
    </Tooltip>
  );
}
