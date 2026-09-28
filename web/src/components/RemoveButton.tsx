import { useEffect, useState } from 'react';
import { CloseIcon } from '@/components/icons';

/**
 * Remove, in two taps: the first turns the button into a red "Remove", the second does it. Sitting
 * beside a link in a list, a single-tap delete is one stray thumb away from losing a record, and a
 * dialog for something this small is heavier than the action. It disarms itself after a few seconds.
 */
export function RemoveButton({ label, onRemove }: { label: string; onRemove: () => Promise<void> | void }) {
  const [armed, setArmed] = useState(false);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!armed) return;
    const timer = setTimeout(() => setArmed(false), 4000);
    return () => clearTimeout(timer);
  }, [armed]);

  async function click() {
    if (!armed) return setArmed(true);
    setBusy(true);
    try {
      await onRemove();
    } finally {
      setBusy(false);
      setArmed(false);
    }
  }

  return (
    <button
      type="button"
      onClick={click}
      onBlur={() => setArmed(false)}
      disabled={busy}
      aria-label={armed ? `Confirm: remove ${label}` : `Remove ${label}`}
      title={armed ? 'Tap Again to Remove' : 'Remove from History'}
      className={`inline-flex h-8 flex-none items-center justify-center rounded-lg text-xs font-semibold transition ${
        armed
          ? 'bg-red-600 px-2.5 text-white hover:bg-red-500'
          : 'w-8 text-slate-400 hover:bg-slate-200/70 hover:text-slate-700 dark:text-slate-500 dark:hover:bg-slate-800 dark:hover:text-slate-200'
      }`}>
      {armed ? 'Remove' : <CloseIcon className="size-3.5" />}
    </button>
  );
}
