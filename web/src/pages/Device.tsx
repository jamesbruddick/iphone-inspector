import { useCallback, useEffect, useRef, useState } from 'react';
import { Link, useNavigate, useParams, useSearchParams } from 'react-router';
import { DeviceHero } from '@/components/DeviceHero';
import { AdvancedData, DeviceReadout } from '@/components/DeviceReadout';
import { AlertIcon, LockIcon, RefreshIcon } from '@/components/icons';
import { readLabel, whenRead } from '@/lib/dates';
import { forgetReading, keepReading, readingFor } from '@/lib/readings';
import { useStore } from '@/lib/store';
import type { DeviceReading } from '@/lib/types';
import { getHistory, pairDevice, readDevice } from '@/lib/usb';
import { useHead } from '@/lib/use-head';

const TABS = [
  { id: 'details', label: 'Details' },
  { id: 'advanced', label: 'Advanced' },
] as const;
type TabId = (typeof TABS)[number]['id'];

export default function Device() {
  const { udid = '' } = useParams<{ udid: string }>();
  const { devices, loaded, refreshDevices, history, refreshHistory, removeFromHistory } = useStore();
  const navigate = useNavigate();
  const device = devices.find(d => d.udid === udid) ?? null;

  const [reading, setReading] = useState<DeviceReading | null>(() => readingFor(udid));
  const [busy, setBusy] = useState(false);
  const [diskPending, setDiskPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pairing, setPairing] = useState(false);

  // The tab lives in the URL, so a reload or a shared link lands on the same one.
  const [params, setParams] = useSearchParams();
  const tab: TabId = params.get('tab') === 'advanced' ? 'advanced' : 'details';
  const setTab = (next: TabId) => setParams(next === 'details' ? {} : { tab: next }, { replace: true });

  useHead({ title: reading?.model ?? device?.model ?? 'iPhone' });

  /**
   * The fast read, then the slow one.
   *
   * Disk usage is the only thing the phone is slow about, so the first read leaves it out and the
   * page fills in at once; the second adds storage when it arrives. `generation` drops the answer
   * of a read that a newer one, an unplug, or a switch to another phone has already replaced.
   */
  const generation = useRef(0);
  const read = useCallback(async () => {
    const mine = ++generation.current;
    const current = () => generation.current === mine;
    setBusy(true);
    setDiskPending(false);
    setError(null);
    try {
      const fast = await readDevice(udid);
      if (!current()) return;
      keepReading(fast);
      setReading(fast);
      setBusy(false);
      setDiskPending(true);
      const full = await readDevice(udid, { disk: true });
      if (!current()) return;
      keepReading(full);
      // The service saved this full read to the history; bring the sidebar up to date.
      refreshHistory();
      setReading(full);
    } catch (err) {
      if (current()) setError(err instanceof Error ? err.message : 'Could not read the iPhone.');
    } finally {
      if (current()) {
        setBusy(false);
        setDiskPending(false);
      }
    }
  }, [udid, refreshHistory]);

  // Whenever this phone is (re)connected and trusted: show what we already have for it, if anything,
  // and read it fresh - the charge, the storage and the settings may all have changed since.
  // Not plugged in: show its last full read from the history, if it has one.
  const connected = device !== null;
  const [missing, setMissing] = useState(false);
  useEffect(() => {
    setMissing(false);
    if (connected || readingFor(udid)) return;
    let cancelled = false;
    getHistory(udid)
      .then(saved => {
        if (cancelled) return;
        keepReading(saved);
        setReading(saved);
      })
      .catch(() => !cancelled && setMissing(true));
    return () => {
      cancelled = true;
    };
  }, [udid, connected]);

  const savedAt = history.find(h => h.udid === udid)?.savedAt ?? null;

  async function forget() {
    await removeFromHistory(udid);
    forgetReading(udid);
    navigate('/', { replace: true });
  }

  const paired = device?.paired ?? false;
  useEffect(() => {
    setReading(readingFor(udid));
    setError(null);
    if (paired) read();
    return () => {
      // Abandon anything in flight, and reset its state with it - otherwise a read cut short by an
      // unplug leaves the button stuck on "Reading…" when the phone comes back.
      generation.current++;
      setBusy(false);
      setDiskPending(false);
    };
  }, [udid, paired, read]);

  async function pair() {
    setPairing(true);
    setError(null);
    try {
      await pairDevice(udid);
      await refreshDevices();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Pairing failed.');
    } finally {
      setPairing(false);
    }
  }

  if (!device && !reading) {
    return (
      <div className="px-4 py-20 text-center">
        <h1 className="text-xl font-extrabold">{loaded && missing ? 'This iPhone Is Not Connected' : 'Looking for the iPhone…'}</h1>
        {loaded && missing && (
          <p className="mt-2 text-base text-slate-600 dark:text-slate-400">It isn't in the history either. Plug it in and unlock it to read it.</p>
        )}
        <Link to="/" className="mt-5 inline-block text-base font-semibold text-blue-600 hover:underline dark:text-blue-400">
          Back to All iPhones
        </Link>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {!device && loaded && (
        <div className="flex flex-wrap items-center gap-x-4 gap-y-2 rounded-xl border border-slate-200 bg-white px-4 py-2.5 dark:border-slate-800 dark:bg-slate-900">
          <p className="min-w-0 flex-1 text-sm text-slate-600 dark:text-slate-400">
            <span className="font-semibold text-slate-900 dark:text-slate-100">Not connected.</span> These are the stats from its last full read
            {savedAt ? `, ${whenRead(savedAt)}` : ''}.
          </p>
          {savedAt && <RemoveHistoryButton onRemove={forget} />}
        </div>
      )}

      {device && !device.paired && (
        <section className="rounded-2xl border border-blue-200 bg-blue-50/60 p-4 sm:p-5 dark:border-blue-900/60 dark:bg-blue-950/30">
          <div className="flex items-start gap-3">
            <LockIcon className="mt-0.5 size-5 flex-none text-blue-600 dark:text-blue-400" />
            <div>
              <h1 className="text-lg font-extrabold tracking-tight">Trust This Computer</h1>
              <p className="mt-1 text-sm leading-relaxed text-slate-600 dark:text-slate-400">
                Unlock the iPhone and tap <span className="font-semibold">Trust</span> when it asks. If the prompt doesn't appear, press Pair.
              </p>
              <button
                type="button"
                onClick={pair}
                disabled={pairing}
                className="mt-3 inline-flex min-h-10 items-center rounded-lg bg-blue-600 px-4 text-sm font-semibold text-white shadow-sm transition hover:bg-blue-500 disabled:opacity-60">
                {pairing ? 'Waiting for Trust on the iPhone…' : 'Pair'}
              </button>
            </div>
          </div>
        </section>
      )}

      {error && (
        <p
          role="alert"
          className="flex gap-2 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700 dark:border-red-900/60 dark:bg-red-950/30 dark:text-red-400">
          <AlertIcon className="mt-0.5 size-4 flex-none" />
          <span className="min-w-0 flex-1">{error}</span>
          {device?.paired && !busy && (
            <button type="button" onClick={read} className="flex-none font-semibold underline underline-offset-2 hover:no-underline">
              Try Again
            </button>
          )}
        </p>
      )}

      {reading ? (
        <>
          <DeviceHero reading={reading} diskPending={diskPending} live={device !== null} />

          <div className="flex items-center gap-3 border-b border-slate-200 dark:border-slate-800">
            <div role="tablist" aria-label="Device information" className="-mb-px flex gap-1">
              {TABS.map(t => (
                <button
                  key={t.id}
                  type="button"
                  role="tab"
                  id={`tab-${t.id}`}
                  aria-selected={tab === t.id}
                  aria-controls={`panel-${t.id}`}
                  onClick={() => setTab(t.id)}
                  className={`min-h-11 border-b-2 px-3 text-sm font-semibold transition ${
                    tab === t.id
                      ? 'border-blue-600 text-slate-900 dark:border-blue-400 dark:text-slate-100'
                      : 'border-transparent text-slate-500 hover:text-slate-800 dark:text-slate-400 dark:hover:text-slate-200'
                  }`}>
                  {t.label}
                </button>
              ))}
            </div>

            <div className="ml-auto flex items-center gap-3 pb-1">
              <span className="hidden text-xs text-slate-500 sm:inline dark:text-slate-400">{readLabel(reading.readAt)}</span>
              {device?.paired && (
                <button
                  type="button"
                  onClick={read}
                  disabled={busy}
                  className="inline-flex min-h-9 items-center gap-1.5 rounded-lg border border-slate-200 bg-white px-3 text-sm font-semibold text-slate-700 transition hover:bg-slate-50 disabled:opacity-50 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-200 dark:hover:bg-slate-800">
                  <RefreshIcon className={`size-4 ${busy ? 'animate-spin' : ''}`} />
                  {busy ? 'Reading…' : 'Refresh'}
                </button>
              )}
            </div>
          </div>

          <div role="tabpanel" id={`panel-${tab}`} aria-labelledby={`tab-${tab}`}>
            {tab === 'details' ? <DeviceReadout reading={reading} /> : <AdvancedData reading={reading} />}
          </div>
        </>
      ) : (
        device?.paired &&
        !error && (
          <p className="py-16 text-center text-base text-slate-500 dark:text-slate-400" aria-live="polite">
            Reading the iPhone…
          </p>
        )
      )}
    </div>
  );
}

/** "Remove from history", in two taps like the sidebar's, but with its words spelled out. */
function RemoveHistoryButton({ onRemove }: { onRemove: () => Promise<void> }) {
  const [armed, setArmed] = useState(false);
  useEffect(() => {
    if (!armed) return;
    const timer = setTimeout(() => setArmed(false), 4000);
    return () => clearTimeout(timer);
  }, [armed]);
  return (
    <button
      type="button"
      onClick={() => (armed ? onRemove() : setArmed(true))}
      onBlur={() => setArmed(false)}
      className={`inline-flex min-h-9 flex-none items-center rounded-lg px-3 text-sm font-semibold transition ${
        armed
          ? 'bg-red-600 text-white hover:bg-red-500'
          : 'border border-slate-200 text-slate-700 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800'
      }`}>
      {armed ? 'Tap Again to Remove' : 'Remove from History'}
    </button>
  );
}
