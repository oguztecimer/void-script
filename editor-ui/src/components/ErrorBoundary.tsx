import { Component, type ErrorInfo, type ReactNode } from 'react';

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error('ErrorBoundary caught:', error, info.componentStack);
  }

  render(): ReactNode {
    if (this.state.hasError) {
      return (
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            height: '100%',
            padding: '2rem',
            color: '#e0e0e0',
            backgroundColor: '#1e1e1e',
            fontFamily: 'monospace',
          }}
        >
          <h2 style={{ marginBottom: '1rem', color: '#f44' }}>Something went wrong</h2>
          <p style={{ marginBottom: '1.5rem', maxWidth: '600px', textAlign: 'center', opacity: 0.8 }}>
            {this.state.error?.message || 'An unexpected error occurred.'}
          </p>
          <button
            onClick={() => window.location.reload()}
            style={{
              padding: '0.5rem 1.5rem',
              background: '#333',
              color: '#e0e0e0',
              border: '1px solid #555',
              borderRadius: '4px',
              cursor: 'pointer',
              fontFamily: 'monospace',
              fontSize: '0.9rem',
            }}
          >
            Reload
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}
