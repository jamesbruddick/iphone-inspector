import { type ReactNode, useCallback, useEffect, useRef, useState } from 'react';
import { CheckIcon, CopyIcon } from '@/components/icons';

/**
 * Put text on the clipboard.
 *
 * The async Clipboard API is the whole story on a secure origin, which localhost is - but it also
 * rejects when the document is not focused, which happens the moment a click lands while devtools
 * or another window has focus. The old selection trick still works in that case, so it stays as a
 * fallback rather than leaving the inspector wondering why nothing copied.
 */
async function write(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    try {
      const field = document.createElement('textarea');
      field.value = text;
      field.setAttribute('readonly', '');
      field.style.position = 'fixed';
      field.style.opacity = '0';
      document.body.append(field);
      field.select();
      const ok = document.execCommand('copy');
      field.remove();
      return ok;
    } catch {
      return false;
    }
  }
}

/** Copy state that resets itself, and never sets state after the component has gone. */
function useCopyState(): [boolean, (text: string) => void] {
  const [copied, setCopied] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => () => void (timer.current && clearTimeout(timer.current)), []);

  const copy = useCallback((text: string) => {
    write(text).then(ok => {
      if (!ok) return;
      setCopied(true);
      if (timer.current) clearTimeout(timer.current);
      timer.current = setTimeout(() => setCopied(false), 1200);
    });
  }, []);

  return [copied, copy];
}

/**
 * A value that copies itself when clicked.
 *
 * No icon and no chrome: a page of facts should not turn into a page of buttons, and the whole
 * value is the target rather than a glyph beside it. The only affordance until the pointer lands is
 * the cursor; the confirmation is the value itself flashing in place, in the same blue the shell
 * uses for live USB state, which the inspector is already looking at - no toast, nothing moves.
 */
export function CopyValue({ value, label, className = '', children }: { value: string; label: string; className?: string; children?: ReactNode }) {
  const [copied, copy] = useCopyState();

  return (
    <button
      type="button"
      onClick={() => copy(value)}
      title={`Copy ${label}`}
      aria-label={`Copy ${label}`}
      className={`-mx-1 cursor-pointer rounded px-1 text-left transition-colors duration-150 hover:bg-slate-100 dark:hover:bg-slate-800 ${
        copied ? 'bg-blue-50 text-blue-700 dark:bg-blue-950/60 dark:text-blue-400' : ''
      } ${className}`}>
      {children ?? value}
      <span className="sr-only" role="status">
        {copied ? `${label} copied` : ''}
      </span>
    </button>
  );
}

/** The corner button on a block of data: copies the lot. */
export function CopyAll({ value, label, className = '' }: { value: string; label: string; className?: string }) {
  const [copied, copy] = useCopyState();

  return (
    <button
      type="button"
      onClick={() => copy(value)}
      aria-label={`Copy ${label}`}
      className={`inline-flex items-center gap-1.5 rounded-lg border px-2 py-1 text-xs font-semibold transition ${className}`}>
      {copied ? <CheckIcon className="size-3.5" /> : <CopyIcon className="size-3.5" />}
      {copied ? 'Copied' : 'Copy'}
    </button>
  );
}
