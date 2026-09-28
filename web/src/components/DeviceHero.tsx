import { BatteryStatus } from '@/components/BatteryStatus';
import { CopyValue } from '@/components/Copy';
import { FinishPicker } from '@/components/FinishPicker';
import { PhoneArt } from '@/components/PhoneArt';
import { setFinishChoice, useFinishChoice } from '@/lib/finish-choice';
import { finishesFor } from '@/lib/finishes';
import { finishKey } from '@/lib/readings';
import type { DeviceReading } from '@/lib/types';

/** Apple's activation states, in plain words. */
const ACTIVATION: Record<string, string> = {
  Activated: 'Activated',
  Unactivated: 'Not Activated',
  FactoryActivated: 'Factory Activated',
  SoftActivation: 'Partially Activated',
  MismatchedIMEI: 'IMEI Mismatch',
  MismatchedICCID: 'SIM Mismatch',
  MissingSIM: 'No SIM',
};

/** Non-breaking spaces, so "169 GB" never wraps between the number and its unit. */
const nb = (text: string) => text.replace(/ /g, '\u00a0');
const gb = (bytes: number) => `${(bytes / 1e9).toFixed(bytes >= 1e11 ? 0 : 1)} GB`;
const plural = (n: number, one: string, many: string) => `${n.toLocaleString('en-US')} ${n === 1 ? one : many}`;

interface Tile {
  label: string;
  value: string;
  /** Lines of context under the value; empty ones are dropped. */
  details?: (string | null | undefined)[];
  /** A 0-100 bar under the value, and its color. */
  bar?: { pct: number; tone: string };
  /** Still being read - shown dimmed rather than as a value. */
  pending?: boolean;
  /** A short note on the value's own line, right-aligned - e.g. the charge cycles beside health. */
  aside?: string | null;
}

/** Green when healthy, amber as it wears, red once it is due for a replacement. */
function healthTone(health: number): string {
  if (health >= 90) return 'bg-emerald-500';
  if (health >= 80) return 'bg-amber-500';
  return 'bg-red-500';
}

function tilesFor(r: DeviceReading, diskPending: boolean): Tile[] {
  const tiles: Tile[] = [];
  const mah = (n: number) => n.toLocaleString('en-US');

  // Health, what the battery holds now against what it held new, and how hard it has been used.
  const capacity =
    r.fullChargeCapacity !== null && r.designCapacity !== null
      ? `${mah(r.fullChargeCapacity)}\u00a0mAh / ${mah(r.designCapacity)}\u00a0mAh`
      : r.fullChargeCapacity !== null
        ? `${mah(r.fullChargeCapacity)} mAh`
        : null;
  const cycles = r.cycleCount !== null ? plural(r.cycleCount, 'Cycle', 'Cycles') : null;
  tiles.push(
    r.batteryHealth !== null
      ? { label: 'Battery', value: `${r.batteryHealth}%`, aside: cycles, bar: { pct: r.batteryHealth, tone: healthTone(r.batteryHealth) }, details: [capacity] }
      : { label: 'Battery', value: 'Not Reported', aside: cycles, details: [capacity] },
  );

  if (r.storageBytes !== null && r.freeBytes !== null && r.storageBytes > 0) {
    const used = r.storageBytes - r.freeBytes;
    const pct = Math.round((used / r.storageBytes) * 100);
    const total = r.storage ?? gb(r.storageBytes);
    tiles.push({
      label: 'Storage',
      value: total,
      bar: { pct, tone: pct >= 90 ? 'bg-red-500' : 'bg-blue-500' },
      details: [`${nb(gb(used))} / ${nb(total)}\u00a0Used`],
    });
  } else if (diskPending) {
    tiles.push({ label: 'Storage', value: 'Reading…', pending: true, details: ['iOS Can Take a Moment to Count'] });
  } else if (r.storage) {
    tiles.push({ label: 'Storage', value: r.storage });
  }

  if (r.iosVersion) tiles.push({ label: 'Software', value: `iOS ${r.iosVersion}`, details: [r.buildVersion && `Build ${r.buildVersion}`] });

  if (r.partNumber) tiles.push({ label: 'Model Number', value: r.partNumber, details: [r.productType] });

  if (r.language) tiles.push({ label: 'Language & Region', value: r.language, details: [r.timeZone] });

  const unit = r.unitShort ?? r.unitType;
  if (unit || r.region) tiles.push({ label: 'Unit', value: unit ?? 'Unknown', details: [r.region] });

  if (r.activation) {
    const findMy = r.findMy === null ? null : `Find My ${r.findMy ? 'On' : 'Off'}`;
    const managed = r.supervised ? 'Supervised' : null;
    tiles.push({ label: 'Activation', value: ACTIVATION[r.activation] ?? r.activation, details: [findMy, managed] });
  }

  if (r.passcodeSet !== null) tiles.push({ label: 'Passcode', value: r.passcodeSet ? 'Set' : 'Not Set' });

  return tiles;
}

function StatTile({ tile }: { tile: Tile }) {
  const details = (tile.details ?? []).filter((d): d is string => !!d);
  return (
    <div className="min-w-0 rounded-xl bg-slate-50 px-2.5 py-2.5 sm:px-3 dark:bg-slate-800/50">
      <p className="text-2xs font-bold tracking-[0.07em] text-balance text-slate-500 uppercase dark:text-slate-400">{tile.label}</p>
      <div className="mt-0.5 flex min-w-0 items-baseline justify-between gap-2">
        <p
          className={`min-w-0 truncate text-lg leading-snug font-extrabold tracking-tight tabular-nums ${tile.pending ? 'font-semibold text-slate-400 italic dark:text-slate-500' : ''}`}
          title={tile.value}>
          {tile.value}
        </p>
        {tile.aside && <p className="flex-none text-xs text-slate-500 tabular-nums dark:text-slate-400">{tile.aside}</p>}
      </div>
      {tile.bar && (
        // Decorative: the value above says the same thing in words.
        <div className="mt-1.5 h-1.5 overflow-hidden rounded-full bg-slate-200 dark:bg-slate-700" aria-hidden="true">
          <div className={`h-full rounded-full ${tile.bar.tone}`} style={{ width: `${Math.max(0, Math.min(100, tile.bar.pct))}%` }} />
        </div>
      )}
      {details.length > 0 && (
        <div className="mt-1 space-y-0.5">
          {details.map(detail => (
            <p key={detail} className="text-xs tracking-tight break-words text-slate-500 tabular-nums dark:text-slate-400">
              {detail}
            </p>
          ))}
        </div>
      )}
    </div>
  );
}

/** The phone's color: what it reported, or the finishes to pick from when it would not say. */
function ColorRow({ reading: r }: { reading: DeviceReading }) {
  const key = finishKey(r.udid, r.serial);
  const picked = useFinishChoice(key, r.model);
  const finishes = finishesFor(r.model);

  if (r.color) {
    return (
      <span className="inline-flex items-center gap-2 text-sm font-semibold">
        {r.colorHex && (
          <span className="size-5 flex-none rounded-full ring-1 ring-black/15 ring-inset dark:ring-white/25" style={{ backgroundColor: r.colorHex }} />
        )}
        {r.color}
      </span>
    );
  }
  if (finishes.length === 0) return <span className="text-sm text-slate-500 dark:text-slate-400">Not Reported</span>;
  return <FinishPicker finishes={finishes} selected={picked} onChange={f => setFinishChoice(key, f?.name ?? null)} />;
}

/** `live` is false for a phone shown from the history, whose charge level would be long out of date. */
export function DeviceHero({ reading: r, diskPending, live }: { reading: DeviceReading; diskPending: boolean; live: boolean }) {
  const picked = useFinishChoice(finishKey(r.udid, r.serial), r.model);
  const hex = r.colorHex ?? picked?.hex ?? null;
  const colorName = r.color ?? picked?.name ?? null;

  const battery = live && r.batteryLevel !== null ? <BatteryStatus level={r.batteryLevel} charging={r.charging} /> : null;
  const ids = [
    { label: 'SN', value: r.serial },
    { label: 'IMEI', value: r.imei },
    { label: 'IMEI 2', value: r.imei2 !== r.imei ? r.imei2 : null },
  ].filter((x): x is { label: string; value: string } => !!x.value);

  // Storage is missing from the fast read by design; it is not "unavailable" until the slow one says
  // so. Color has its own row; Find My and management are only worth a mention when reported.
  const unavailable = r.unavailable.filter(u => !['Color', 'Find My', 'Device Management'].includes(u) && !(diskPending && u === 'Storage Capacity'));

  return (
    <section className="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm sm:p-5 dark:border-slate-800 dark:bg-slate-900">
      <div className="flex gap-4 sm:gap-5">
        <div className="flex-none self-start rounded-xl bg-slate-100 p-2 sm:p-2.5 dark:bg-slate-800/60">
          <PhoneArt
            model={r.model}
            hex={hex}
            label={`Illustration of ${r.model}${colorName ? ` in ${colorName}` : ''}`}
            className="h-24 w-auto drop-shadow-sm sm:h-32"
          />
        </div>

        <div className="min-w-0 flex-1">
          <div className="flex items-start justify-between gap-4">
            <h1 className="text-xl leading-tight font-extrabold tracking-tight text-balance sm:text-2xl">{r.model}</h1>
            {battery && <div className="hidden flex-none pt-1 sm:block">{battery}</div>}
          </div>
          {/* The phone's own name, when it has one. On a phone the battery sits at the end of this
              line, since beside the title it would squeeze the model name onto two lines. */}
          {(r.deviceName || battery) && (
            <div className="mt-0.5 flex items-center gap-3">
              {r.deviceName && <p className="min-w-0 truncate text-sm text-slate-500 dark:text-slate-400">{r.deviceName}</p>}
              {battery && <div className="flex-none sm:hidden">{battery}</div>}
            </div>
          )}

          {ids.length > 0 && (
            <dl className="mt-2 flex flex-col gap-0.5 text-2xs font-bold tracking-[0.06em] text-slate-400 uppercase sm:flex-row sm:flex-wrap sm:gap-x-4 dark:text-slate-500">
              {ids.map(item => (
                <div key={item.label} className="flex items-baseline gap-1">
                  <dt>{item.label}</dt>
                  <dd>
                    <CopyValue
                      value={item.value}
                      label={item.label}
                      className="font-semibold tracking-normal text-slate-600 tabular-nums dark:text-slate-300"
                    />
                  </dd>
                </div>
              ))}
            </dl>
          )}

          {/* On a phone the swatches get the full card width below; beside the art they would wrap. */}
          <div className="mt-3 hidden items-center gap-3 sm:flex">
            <span className="text-2xs font-bold tracking-[0.07em] text-slate-500 uppercase dark:text-slate-400">Color</span>
            <ColorRow reading={r} />
          </div>
        </div>
      </div>

      <div className="mt-3 flex flex-col gap-2 sm:hidden">
        <span className="text-2xs font-bold tracking-[0.07em] text-slate-500 uppercase dark:text-slate-400">Color</span>
        <ColorRow reading={r} />
      </div>

      <div className="mt-4 grid grid-cols-2 gap-2 sm:gap-3 lg:grid-cols-4">
        {tilesFor(r, diskPending).map(tile => (
          <StatTile key={tile.label} tile={tile} />
        ))}
      </div>

      {unavailable.length > 0 && <p className="mt-3 text-xs text-slate-500 dark:text-slate-400">Not reported by this phone: {unavailable.join(', ')}</p>}
    </section>
  );
}
