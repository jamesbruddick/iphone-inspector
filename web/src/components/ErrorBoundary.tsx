import { Component, type ErrorInfo, type ReactNode } from 'react';

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  override state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  override componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('Uncaught render error:', error, info.componentStack);
  }

  override render() {
    if (this.state.error) {
      return (
        <main className="flex min-h-screen items-center justify-center bg-slate-50 px-4 text-slate-900 dark:bg-slate-950 dark:text-slate-100">
          <div className="max-w-sm rounded-xl border border-red-200 bg-red-50 p-6 text-center dark:border-red-900/60 dark:bg-red-950/40">
            <h1 className="text-lg font-semibold text-red-800 dark:text-red-300">Something Went Wrong</h1>
            <p className="mt-2 text-sm text-red-700 dark:text-red-400">{this.state.error.message}</p>
            <p className="mt-3 text-xs text-red-600 dark:text-red-500">Reload the page to carry on.</p>
          </div>
        </main>
      );
    }

    return this.props.children;
  }
}
