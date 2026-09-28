/**
 * The finish someone picked for a phone that would not report its own color.
 *
 * Kept in this browser's storage, keyed by serial number (or the device ID before the phone is
 * trusted), so the choice survives a reload and a re-plug. Storage can be unavailable - a private
 * window, blocked site data - in which case the pick simply lasts until the page closes.
 */

import { useSyncExternalStore } from 'react';
import { type Finish, finishesFor } from './finishes';

const PREFIX = 'iphone-inspector:finish:';
const memory = new Map<string, string>();
const listeners = new Set<() => void>();

function read(key: string): string | null {
  try {
    return localStorage.getItem(PREFIX + key) ?? memory.get(key) ?? null;
  } catch {
    return memory.get(key) ?? null;
  }
}

export function setFinishChoice(key: string, name: string | null) {
  if (name === null) memory.delete(key);
  else memory.set(key, name);
  try {
    if (name === null) localStorage.removeItem(PREFIX + key);
    else localStorage.setItem(PREFIX + key, name);
  } catch {
    // Storage is off; the in-memory copy above still holds the choice for this page.
  }
  for (const notify of listeners) notify();
}

function subscribe(notify: () => void) {
  listeners.add(notify);
  // Another tab picking a finish for the same phone.
  window.addEventListener('storage', notify);
  return () => {
    listeners.delete(notify);
    window.removeEventListener('storage', notify);
  };
}

/** The picked finish for this phone, if one was picked and still belongs to its model. */
export function useFinishChoice(key: string | null, model: string | null): Finish | null {
  const name = useSyncExternalStore(subscribe, () => (key ? read(key) : null));
  if (!name || !model) return null;
  return finishesFor(model).find(f => f.name === name) ?? null;
}
