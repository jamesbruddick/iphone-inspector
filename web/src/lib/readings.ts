/**
 * The last reading of each phone seen by this page, in memory.
 *
 * It lets a phone's page show its stats at once when you come back to it, while a fresh read runs,
 * and lets the device list draw each phone in its real color. The lasting copy is the history,
 * which the service keeps on disk.
 */

import type { DeviceReading } from './types';

const readings = new Map<string, DeviceReading>();

export const readingFor = (udid: string): DeviceReading | null => readings.get(udid) ?? null;

export function keepReading(reading: DeviceReading) {
  readings.set(reading.udid, reading);
}

export function forgetReading(udid: string) {
  readings.delete(udid);
}

/** What a picked finish is stored under: the serial, which outlives the device ID across machines. */
export const finishKey = (udid: string, serial: string | null | undefined) => serial ?? udid;
