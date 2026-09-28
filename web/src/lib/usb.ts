/**
 * Client for the local service. Every call goes to 127.0.0.1 - the same origin the app is served
 * from - so no device data crosses the network.
 */

import type { DeviceReading, HistoryItem } from './types';

export interface ConnectedDevice {
  udid: string;
  name: string;
  model: string | null;
  ios: string | null;
  serial: string | null;
  /** False when the phone has not yet tapped Trust on this computer. */
  paired: boolean;
}

export interface DeviceList {
  devices: ConnectedDevice[];
  /** Set when the system's iPhone USB service is missing or failing. */
  toolError: string | null;
}

async function call<T>(path: string, init?: RequestInit): Promise<T> {
  let response: Response;
  try {
    response = await fetch(path, init);
  } catch {
    throw new Error('iPhone Inspector is not running. Start it from its tray icon or launch the app again.');
  }
  const body = await response.json().catch(() => null);
  if (!response.ok) {
    throw new Error((body as { error?: string } | null)?.error ?? `Request failed (${response.status})`);
  }
  return body as T;
}

export const listDevices = () => call<DeviceList>('/api/devices');
export const pairDevice = (udid: string) => call<{ message: string }>(`/api/devices/${udid}/pair`, { method: 'POST' });
/**
 * Read a phone.
 *
 * Without `disk`, the storage figures are left out. That is the fast read - well under a second -
 * because iOS computes disk usage on demand and the first request after a phone is plugged in can
 * take the best part of a minute. Ask for it once the page is already showing everything else.
 */
export const readDevice = (udid: string, options: { disk?: boolean } = {}) => call<DeviceReading>(`/api/devices/${udid}/read${options.disk ? '?disk=1' : ''}`);

export const listHistory = () => call<{ devices: HistoryItem[] }>('/api/history');
export const getHistory = (udid: string) => call<DeviceReading & { savedAt: string }>(`/api/history/${udid}`);
export async function removeHistory(udid: string): Promise<void> {
  const response = await fetch(`/api/history/${udid}`, { method: 'DELETE' }).catch(() => null);
  if (!response || (!response.ok && response.status !== 404)) throw new Error('Could not remove it from the history.');
}
