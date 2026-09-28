import { useEffect, useState } from 'react';
import { Link, NavLink, Outlet, useLocation } from 'react-router';
import { CloseIcon, MenuIcon } from '@/components/icons';
import { Sidebar } from '@/components/Sidebar';

/**
 * Two-pane shell: a device rail that stays put and a working surface that scrolls.
 *
 * Below `lg` the rail becomes a drawer, because at phone width the device stats need the
 * whole screen and the list is navigation, not context.
 */
export function AppShell() {
  const [drawerOpen, setDrawerOpen] = useState(false);
  // The router's location, not window.location: under hash routing window.location.pathname never
  // changes, so watching it would leave the drawer open on every navigation.
  const { pathname } = useLocation();

  // Navigating is the drawer's job; once it has done it, get out of the way. The pathname is a
  // trigger rather than a value read in the body, which is what the rule cannot tell apart -
  // dropping it would leave the drawer open on back and forward navigation.
  // biome-ignore lint/correctness/useExhaustiveDependencies: pathname is the trigger, by design.
  useEffect(() => setDrawerOpen(false), [pathname]);

  useEffect(() => {
    if (!drawerOpen) return;
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && setDrawerOpen(false);
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [drawerOpen]);

  return (
    <div className="min-h-dvh bg-slate-100 text-base text-slate-900 antialiased dark:bg-slate-950 dark:text-slate-100">
      <header className="no-print sticky top-0 z-30 border-b border-slate-200 bg-white/85 pt-[env(safe-area-inset-top)] backdrop-blur-md dark:border-slate-800 dark:bg-slate-900/85">
        <div className="mx-auto flex h-14 max-w-[1400px] items-center gap-3 px-4 sm:px-6">
          <button
            type="button"
            onClick={() => setDrawerOpen(true)}
            aria-label="Show iPhones"
            aria-expanded={drawerOpen}
            className="-ml-2 inline-flex size-10 items-center justify-center rounded-lg text-slate-600 transition hover:bg-slate-200/70 lg:hidden dark:text-slate-300 dark:hover:bg-slate-800">
            <MenuIcon className="size-[18px]" />
          </button>

          {/* The mark holds its own weight; the name does not need to shout beside it. */}
          <Link to="/" className="flex items-center gap-2.5 rounded-lg" aria-label="iPhone Inspector, home">
            <svg className="size-5 flex-none text-slate-900 dark:text-slate-100" viewBox="0 0 24 24" aria-hidden="true">
              <rect x="6" y="2.5" width="12" height="19" rx="3.4" fill="none" stroke="currentColor" strokeWidth="1.7" />
              <path
                d="M9.3 12.3l2.1 2.1 3.5-4.2"
                fill="none"
                className="stroke-blue-600 dark:stroke-blue-400"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
            <span className="text-base tracking-[-0.012em] whitespace-nowrap">
              <span className="font-medium text-slate-500 dark:text-slate-400">iPhone</span>{' '}
              <span className="font-extrabold text-slate-900 dark:text-slate-100">Inspector</span>
            </span>
          </Link>

          <div className="ml-auto flex items-center gap-2">
            <NavLink
              to="/about"
              className={({ isActive }) =>
                `rounded-lg px-3 py-2 text-sm font-semibold transition ${
                  isActive
                    ? 'text-slate-900 dark:text-slate-100'
                    : 'text-slate-500 hover:bg-slate-100 hover:text-slate-900 dark:text-slate-400 dark:hover:bg-slate-800/70 dark:hover:text-slate-100'
                }`
              }>
              About
            </NavLink>
          </div>
        </div>
      </header>

      <div className="mx-auto max-w-[1400px] lg:grid lg:grid-cols-[272px_minmax(0,1fr)] lg:gap-6 lg:px-6">
        {/* 3.5rem of header plus its 1px bottom border. Leave the border out and the page is
            permanently 1px taller than the viewport - a scrollbar on every screen, forever. */}
        <aside className="no-print hidden lg:sticky lg:top-[calc(3.5rem+1px)] lg:block lg:h-[calc(100dvh-3.5rem-1px)] lg:py-5">
          <Sidebar />
        </aside>

        <main className="min-w-0 px-4 pt-5 pb-[max(1.5rem,env(safe-area-inset-bottom))] sm:px-6 lg:px-0 lg:py-5">
          <Outlet />
        </main>
      </div>

      {/* Mobile drawer */}
      {drawerOpen && (
        <div className="no-print fixed inset-0 z-40 lg:hidden">
          <button
            type="button"
            aria-label="Close iPhone List"
            onClick={() => setDrawerOpen(false)}
            className="absolute inset-0 bg-slate-950/50 backdrop-blur-[2px]"
          />
          <div className="absolute inset-y-0 left-0 flex w-[min(86vw,320px)] flex-col border-r border-slate-200 bg-slate-100 pt-[env(safe-area-inset-top)] shadow-2xl dark:border-slate-800 dark:bg-slate-950">
            <div className="flex items-center justify-between px-4 py-3">
              <span className="text-lg font-extrabold tracking-tight">iPhones</span>
              <button
                type="button"
                onClick={() => setDrawerOpen(false)}
                aria-label="Close iPhone List"
                className="inline-flex size-10 items-center justify-center rounded-lg text-slate-600 transition hover:bg-slate-200/70 dark:text-slate-300 dark:hover:bg-slate-800">
                <CloseIcon className="size-[18px]" />
              </button>
            </div>
            <div className="min-h-0 flex-1 overflow-hidden px-3 pb-4">
              <Sidebar onNavigate={() => setDrawerOpen(false)} />
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
