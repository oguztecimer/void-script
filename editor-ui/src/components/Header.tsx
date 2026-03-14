import { useState } from 'react';
import { useStore } from '../state/store';
import { sendToRust } from '../ipc/bridge';

export function Header() {
  const activeTabId = useStore((s) => s.activeTabId);
  const tabs = useStore((s) => s.tabs);
  const activeTab = tabs.find((t) => t.scriptId === activeTabId);
  const isRunning = useStore((s) => s.isRunning);
  const isDebugging = useStore((s) => s.isDebugging);
  const isPaused = useStore((s) => s.isPaused);
  const active = isRunning || isDebugging;

  return (
    <div className="titlebar-drag" style={{
      display: 'flex',
      alignItems: 'center',
      height: '40px',
      backgroundColor: 'var(--bg-toolbar)',
      padding: '0 8px',
      userSelect: 'none',
      borderBottom: '1px solid var(--border-strong)',
      fontSize: '13px',
    }}>
      {/* macOS-style window controls */}
      <WindowControls />

      <Separator />

      {/* Hamburger menu */}
      <ToolBtn title="Menu">
        <svg width="14" height="14" viewBox="0 0 16 16">
          <line x1="2" y1="4" x2="14" y2="4" stroke="currentColor" strokeWidth="1.5"/>
          <line x1="2" y1="8" x2="14" y2="8" stroke="currentColor" strokeWidth="1.5"/>
          <line x1="2" y1="12" x2="14" y2="12" stroke="currentColor" strokeWidth="1.5"/>
        </svg>
      </ToolBtn>

      <Separator />

      {/* Back/Forward navigation */}
      <ToolBtn title="Back">
        <svg width="10" height="10" viewBox="0 0 16 16"><path d="M10 2L4 8l6 6" stroke="currentColor" strokeWidth="2" fill="none"/></svg>
      </ToolBtn>
      <ToolBtn title="Forward">
        <svg width="10" height="10" viewBox="0 0 16 16"><path d="M6 2l6 6-6 6" stroke="currentColor" strokeWidth="2" fill="none"/></svg>
      </ToolBtn>

      <Separator />

      {/* Project widget */}
      <HeaderWidget
        icon={<svg width="12" height="12" viewBox="0 0 16 16"><path d="M2 3h12v2H2V3zm0 4h12v2H2V7zm0 4h8v2H2v-2z" fill="currentColor"/></svg>}
        label="VOID//SCRIPT"
        hasDropdown
      />

      <Separator />

      {/* VCS branch widget */}
      <HeaderWidget
        icon={<svg width="12" height="12" viewBox="0 0 16 16"><circle cx="5" cy="4" r="2" stroke="currentColor" strokeWidth="1.2" fill="none"/><circle cx="11" cy="4" r="2" stroke="currentColor" strokeWidth="1.2" fill="none"/><circle cx="5" cy="12" r="2" stroke="currentColor" strokeWidth="1.2" fill="none"/><path d="M5 6v4M11 6c0 4-6 4-6 4" stroke="currentColor" strokeWidth="1.2" fill="none"/></svg>}
        label="main"
        muted
      />

      {/* Center spacer - draggable */}
      <div style={{ flex: 1 }} />

      {/* Right: run config + run/debug */}
      <div className="titlebar-no-drag" style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
        {/* Run configuration selector */}
        <RunConfigSelector label={activeTab ? `${activeTab.name}.vs` : 'No configuration'} />

        <Separator />

        {/* Run/Debug/Stop buttons */}
        {!active ? (
          <>
            <ActionBtn
              title="Run"
              iconColor="var(--icon-run)"
              bgColor="var(--bg-btn-run)"
              hoverBg="var(--bg-btn-run-hover)"
              onClick={() => activeTabId && sendToRust({ type: 'run_script', script_id: activeTabId })}
              disabled={!activeTabId}
            >
              <svg width="10" height="10" viewBox="0 0 16 16"><path d="M4 2l10 6-10 6V2z" fill="currentColor"/></svg>
            </ActionBtn>
            <ActionBtn
              title="Debug"
              iconColor="var(--icon-debug)"
              bgColor="var(--bg-btn-debug)"
              hoverBg="var(--bg-btn-debug-hover)"
              onClick={() => activeTabId && sendToRust({ type: 'debug_start', script_id: activeTabId })}
              disabled={!activeTabId}
            >
              <svg width="12" height="12" viewBox="0 0 16 16">
                <circle cx="8" cy="9" r="5" stroke="currentColor" strokeWidth="1.5" fill="none"/>
                <line x1="8" y1="4" x2="6" y2="1" stroke="currentColor" strokeWidth="1.5"/>
                <line x1="8" y1="4" x2="10" y2="1" stroke="currentColor" strokeWidth="1.5"/>
                <line x1="3" y1="7" x2="13" y2="7" stroke="currentColor" strokeWidth="1"/>
                <line x1="3" y1="10" x2="13" y2="10" stroke="currentColor" strokeWidth="1"/>
              </svg>
            </ActionBtn>
          </>
        ) : (
          <>
            <ActionBtn
              title="Stop"
              iconColor="var(--icon-stop)"
              bgColor="var(--bg-btn-stop)"
              hoverBg="var(--bg-btn-stop-hover)"
              onClick={() => activeTabId && sendToRust({ type: 'stop_script', script_id: activeTabId })}
            >
              <svg width="8" height="8" viewBox="0 0 10 10"><rect width="10" height="10" rx="1.5" fill="currentColor"/></svg>
            </ActionBtn>
            {isDebugging && isPaused && (
              <>
                <ActionBtn
                  title="Resume"
                  iconColor="var(--icon-run)"
                  bgColor="var(--bg-btn-run)"
                  hoverBg="var(--bg-btn-run-hover)"
                  onClick={() => activeTabId && sendToRust({ type: 'debug_continue', script_id: activeTabId })}
                >
                  <svg width="10" height="10" viewBox="0 0 16 16"><path d="M4 2l10 6-10 6V2z" fill="currentColor"/></svg>
                </ActionBtn>
                <ToolBtn title="Step Over" onClick={() => activeTabId && sendToRust({ type: 'debug_step_over', script_id: activeTabId })}>
                  <svg width="14" height="14" viewBox="0 0 16 16"><path d="M2 12h4V8h4v4h4L8 4z" fill="currentColor" transform="rotate(90 8 8)"/></svg>
                </ToolBtn>
                <ToolBtn title="Step Into" onClick={() => activeTabId && sendToRust({ type: 'debug_step_into', script_id: activeTabId })}>
                  <svg width="14" height="14" viewBox="0 0 16 16"><path d="M8 2v8m-3-3l3 3 3-3M5 14h6" stroke="currentColor" strokeWidth="1.5" fill="none"/></svg>
                </ToolBtn>
                <ToolBtn title="Step Out" onClick={() => activeTabId && sendToRust({ type: 'debug_step_out', script_id: activeTabId })}>
                  <svg width="14" height="14" viewBox="0 0 16 16"><path d="M8 14V6m-3 3l3-3 3 3M5 2h6" stroke="currentColor" strokeWidth="1.5" fill="none"/></svg>
                </ToolBtn>
              </>
            )}
            <Separator />
            <span style={{ color: isDebugging ? 'var(--icon-debug)' : 'var(--icon-run)', fontSize: '11px', padding: '0 4px' }}>
              {isDebugging ? (isPaused ? 'Paused' : 'Debugging...') : 'Running...'}
            </span>
          </>
        )}
      </div>
    </div>
  );
}

/* --- Window Controls (macOS traffic lights) --- */

function WindowControls() {
  return (
    <div className="titlebar-no-drag" style={{
      display: 'flex',
      alignItems: 'center',
      gap: '8px',
      padding: '0 4px',
    }}>
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
  const [hovered, setHovered] = useState(false);

  return (
    <div
      onClick={onClick}
      title={title}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      style={{
        width: '12px',
        height: '12px',
        borderRadius: '50%',
        backgroundColor: color,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        cursor: 'pointer',
        fontSize: '9px',
        fontWeight: 700,
        lineHeight: 1,
        color: hovered ? 'rgba(0,0,0,0.6)' : 'transparent',
      }}
    >
      {hoverSymbol}
    </div>
  );
}

/* --- Separator --- */

function Separator() {
  return (
    <div style={{
      width: '1px',
      height: '16px',
      backgroundColor: 'var(--border-subtle)',
      margin: '0 6px',
      flexShrink: 0,
    }} />
  );
}

/* --- Toolbar icon button --- */

function ToolBtn({ title, onClick, disabled, children }: {
  title: string;
  onClick?: () => void;
  disabled?: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      className="titlebar-no-drag"
      onClick={onClick}
      disabled={disabled}
      title={title}
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        width: '28px',
        height: '28px',
        background: 'none',
        border: 'none',
        borderRadius: '6px',
        color: disabled ? 'var(--text-disabled)' : 'var(--text-secondary)',
        cursor: disabled ? 'default' : 'pointer',
        padding: 0,
        opacity: disabled ? 0.5 : 1,
      }}
      onMouseEnter={(e) => { if (!disabled) e.currentTarget.style.backgroundColor = 'var(--bg-hover)'; }}
      onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = 'transparent'; }}
    >
      {children}
    </button>
  );
}

/* --- Header widget (project, VCS) --- */

function HeaderWidget({ icon, label, muted, hasDropdown }: {
  icon: React.ReactNode;
  label: string;
  muted?: boolean;
  hasDropdown?: boolean;
}) {
  return (
    <button
      className="titlebar-no-drag"
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: '6px',
        height: '26px',
        padding: '0 8px',
        background: 'none',
        border: 'none',
        borderRadius: '6px',
        color: muted ? 'var(--text-secondary)' : 'var(--text-primary)',
        cursor: 'pointer',
        fontSize: '13px',
        fontFamily: 'inherit',
        fontWeight: muted ? 400 : 600,
      }}
      onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = 'var(--bg-hover)'; }}
      onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = 'transparent'; }}
    >
      <span style={{ display: 'flex', alignItems: 'center', color: muted ? 'var(--text-tertiary)' : 'var(--text-secondary)' }}>{icon}</span>
      <span>{label}</span>
      {hasDropdown && (
        <svg width="8" height="8" viewBox="0 0 8 8" style={{ color: 'var(--text-tertiary)' }}>
          <path d="M1 2.5l3 3 3-3" stroke="currentColor" strokeWidth="1.2" fill="none"/>
        </svg>
      )}
    </button>
  );
}

/* --- Run configuration selector --- */

function RunConfigSelector({ label }: { label: string }) {
  return (
    <button
      className="titlebar-no-drag"
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: '6px',
        padding: '0 10px',
        backgroundColor: 'var(--bg-run-config)',
        border: 'none',
        borderRadius: '6px',
        color: 'var(--text-primary)',
        height: '26px',
        cursor: 'pointer',
        fontSize: '13px',
        fontFamily: 'inherit',
      }}
      onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = 'var(--border-subtle)'; }}
      onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = 'var(--bg-run-config)'; }}
    >
      <svg width="10" height="10" viewBox="0 0 16 16" style={{ color: 'var(--text-secondary)' }}>
        <path d="M4 2l10 6-10 6V2z" fill="currentColor"/>
      </svg>
      <span>{label}</span>
      <svg width="8" height="8" viewBox="0 0 8 8" style={{ color: 'var(--text-tertiary)' }}>
        <path d="M1 2.5l3 3 3-3" stroke="currentColor" strokeWidth="1.2" fill="none"/>
      </svg>
    </button>
  );
}

/* --- Action button (Run/Debug/Stop with colored background) --- */

function ActionBtn({ title, iconColor, bgColor, hoverBg, onClick, disabled, children }: {
  title: string;
  iconColor: string;
  bgColor: string;
  hoverBg: string;
  onClick?: () => void;
  disabled?: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      className="titlebar-no-drag"
      onClick={onClick}
      disabled={disabled}
      title={title}
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        width: '28px',
        height: '28px',
        backgroundColor: disabled ? 'transparent' : bgColor,
        border: 'none',
        borderRadius: '6px',
        color: disabled ? 'var(--text-disabled)' : iconColor,
        cursor: disabled ? 'default' : 'pointer',
        padding: 0,
        opacity: disabled ? 0.5 : 1,
      }}
      onMouseEnter={(e) => { if (!disabled) e.currentTarget.style.backgroundColor = hoverBg; }}
      onMouseLeave={(e) => { if (!disabled) e.currentTarget.style.backgroundColor = disabled ? 'transparent' : bgColor; }}
    >
      {children}
    </button>
  );
}
