import { StatusSegment } from '../primitives/StatusSegment';

interface DiagnosticsWidgetProps {
  errorCount: number;
  warningCount: number;
  hasActiveTab: boolean;
}

function ErrorIcon() {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
      <circle cx="5" cy="5" r="4.5" fill="var(--accent-red)" />
      <path d="M3.5 3.5l3 3M6.5 3.5l-3 3" stroke="white" strokeWidth="1.2" strokeLinecap="round"/>
    </svg>
  );
}

function WarningIcon() {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
      <path d="M5 1L9.33 8.5H0.67L5 1z" fill="var(--accent-yellow)" />
      <path d="M5 4.5v1.5" stroke="white" strokeWidth="1.2" strokeLinecap="round"/>
      <circle cx="5" cy="7.2" r="0.5" fill="white"/>
    </svg>
  );
}

export function DiagnosticsWidget({ errorCount, warningCount, hasActiveTab }: DiagnosticsWidgetProps) {
  if (!hasActiveTab) return null;

  return (
    <>
      {errorCount > 0 && (
        <StatusSegment
          icon={<ErrorIcon />}
          label={<span style={{ color: 'var(--accent-red)' }}>{errorCount}</span>}
        />
      )}
      {warningCount > 0 && (
        <StatusSegment
          icon={<WarningIcon />}
          label={<span style={{ color: 'var(--accent-yellow)' }}>{warningCount}</span>}
        />
      )}
      {errorCount === 0 && warningCount === 0 && (
        <StatusSegment label={<span style={{ color: 'var(--accent-green)' }}>BOUND</span>} />
      )}
    </>
  );
}
