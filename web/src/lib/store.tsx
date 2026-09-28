/**
 * The connected-device poll and the history, held once for the whole app so the sidebar and the
 * device page agree.
 */

import { createContext, type ReactNode, use, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { HistoryItem } from './types';
import { type ConnectedDevice, listDevices, listHistory, removeHistory } from './usb';

const POLL_MS = 3000;

interface StoreValue {
  devices: ConnectedDevice[];
  /** False until the first answer from the service. */
  loaded: boolean;
  /** Set when the system's iPhone USB service is missing or failing. */
  toolError: string | null;
  /** True when the local service is unreachable. */
  serviceOffline: boolean;
  refreshDevices: () => Promise<void>;
  /** Every phone read in full on this computer, newest first. */
  history: HistoryItem[];
  refreshHistory: () => Promise<void>;
  removeFromHistory: (udid: string) => Promise<void>;
}

const StoreContext = createContext<StoreValue | null>(null);

export function StoreProvider({ children }: { children: ReactNode }) {
  const [devices, setDevices] = useState<ConnectedDevice[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [toolError, setToolError] = useState<string | null>(null);
  const [serviceOffline, setServiceOffline] = useState(false);
  const [history, setHistory] = useState<HistoryItem[]>([]);

  const refreshHistory = useCallback(async () => {
    try {
      setHistory((await listHistory()).devices);
    } catch {
      // The service is down; the device poll already says so.
    }
  }, []);

  useEffect(() => {
    refreshHistory();
  }, [refreshHistory]);

  const removeFromHistory = useCallback(async (udid: string) => {
    await removeHistory(udid);
    setHistory(current => current.filter(h => h.udid !== udid));
  }, []);

  const refreshDevices = useCallback(async () => {
    try {
      const result = await listDevices();
      setDevices(result.devices);
      setToolError(result.toolError);
      setServiceOffline(false);
    } catch {
      setDevices([]);
      setServiceOffline(true);
    } finally {
      setLoaded(true);
    }
  }, []);

  // Polling pauses while the tab is hidden, so a browser left open overnight is not asking the
  // service about the USB bus every few seconds until morning.
  const timer = useRef<ReturnType<typeof setInterval> | null>(null);
  useEffect(() => {
    const start = () => {
      if (timer.current) return;
      refreshDevices();
      timer.current = setInterval(refreshDevices, POLL_MS);
    };
    const stop = () => {
      if (!timer.current) return;
      clearInterval(timer.current);
      timer.current = null;
    };
    const onVisibility = () => (document.hidden ? stop() : start());

    start();
    document.addEventListener('visibilitychange', onVisibility);
    return () => {
      stop();
      document.removeEventListener('visibilitychange', onVisibility);
    };
  }, [refreshDevices]);

  const value = useMemo<StoreValue>(
    () => ({ devices, loaded, toolError, serviceOffline, refreshDevices, history, refreshHistory, removeFromHistory }),
    [devices, loaded, toolError, serviceOffline, refreshDevices, history, refreshHistory, removeFromHistory],
  );

  return <StoreContext value={value}>{children}</StoreContext>;
}

export function useStore(): StoreValue {
  const value = use(StoreContext);
  if (!value) throw new Error('useStore must be used inside StoreProvider');
  return value;
}
