/** What the local service can read off a connected iPhone. Mirrors `DeviceReading` in `src/reading.rs`. */

export interface DetailRow {
  label: string;
  value: string;
}

export interface DetailSection {
  id: string;
  name: string;
  rows: DetailRow[];
  /** Collapsed by default: technical values most people never need. */
  advanced?: boolean;
}

export interface DeviceReading {
  udid: string;
  readAt: string;
  deviceName: string | null;
  model: string;
  productType: string | null;
  chip: string | null;
  /** Screen size, e.g. `6.9"`. */
  display: string | null;
  released: number | null;
  iosVersion: string | null;
  buildVersion: string | null;
  storage: string | null;
  /** Free space, already formatted, e.g. "113 GB". */
  storageFree: string | null;
  /** Raw byte counts behind `storage` and `storageFree`. */
  storageBytes: number | null;
  freeBytes: number | null;
  color: string | null;
  colorHex: string | null;
  imei: string | null;
  /** The second IMEI on a dual-SIM phone; null on single-IMEI hardware. */
  imei2: string | null;
  serial: string | null;
  batteryHealth: number | null;
  cycleCount: number | null;
  /** Current charge, in percent. */
  batteryLevel: number | null;
  charging: boolean | null;
  /** mAh when new, and the most it holds now. */
  designCapacity: number | null;
  fullChargeCapacity: number | null;
  carrier: string | null;
  /** Apple's part number with its region suffix, e.g. "MJW44LL/A". */
  partNumber: string | null;
  passcodeSet: boolean | null;
  /** SIM state and SIM tray state in words, e.g. "Ready" / "Not Inserted", "Absent". */
  simStatus: string | null;
  simTray: string | null;
  /** The phone's language, short: "English (US)". */
  language: string | null;
  /** e.g. "America/Chicago". */
  timeZone: string | null;
  /** true means Find My is on. */
  findMy: boolean | null;
  /** true means the phone is supervised by an organization. */
  supervised: boolean | null;
  activation: string | null;
  region: string | null;
  unitType: string | null;
  /** One or two words for the summary strip; `unitType` is the full sentence. */
  unitShort: string | null;
  unitLevel: 'ok' | 'warn' | 'fail' | null;
  manufactured: string | null;
  /** Age in words, e.g. "About 4 years" - from the serial where it decodes, else the release year. */
  age: string | null;
  /** What the phone would not tell us, so the UI can say why a value is missing. */
  unavailable: string[];
  /** Every value the phone reported, grouped for display. */
  sections: DetailSection[];
  /** The untouched plist dump behind those sections. */
  raw: Record<string, unknown>;
}

/** A phone in the history: enough to list it, without its whole reading. */
export interface HistoryItem {
  udid: string;
  /** When its last full reading was saved. */
  savedAt: string;
  model: string | null;
  deviceName: string | null;
  serial: string | null;
  iosVersion: string | null;
  colorHex: string | null;
}
