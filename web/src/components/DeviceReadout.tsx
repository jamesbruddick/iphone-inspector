import { CopyAll, CopyValue } from '@/components/Copy';
import type { DeviceReading } from '@/lib/types';

function Panel({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm sm:p-5 dark:border-slate-800 dark:bg-slate-900">
      <h2 className="text-xs font-bold tracking-[0.08em] text-slate-500 uppercase dark:text-slate-400">{title}</h2>
      <div className="mt-2.5">{children}</div>
    </section>
  );
}

/**
 * Label left, value right, wrapping rather than truncating so nothing is unreadable on a phone.
 *
 * The row itself wraps before the value does: a value too long to sit beside its label drops to a
 * line of its own, still right-aligned, rather than being broken mid-phrase into a ragged column -
 * "American English (United / States)" is how a narrow panel mangles a language name otherwise.
 */
/**
 * Label left, value right, wrapping rather than truncating so nothing is unreadable on a phone.
 *
 * The row itself wraps before the value does: a value too long to sit beside its label drops to a
 * line of its own, still right-aligned, rather than being broken mid-phrase into a ragged column.
 */
function Rows({ rows }: { rows: { label: string; value: string }[] }) {
  return (
    <dl className="divide-y divide-slate-100 dark:divide-slate-800">
      {rows.map(row => (
        <div key={row.label} className="flex flex-wrap items-start justify-between gap-x-4 py-1.5">
          <dt className="flex-none text-sm text-slate-500 dark:text-slate-400">{row.label}</dt>
          <dd className="ml-auto min-w-0 text-right text-sm font-semibold">
            <CopyValue value={row.value} label={row.label} />
          </dd>
        </div>
      ))}
    </dl>
  );
}

/**
 * The order the Details tab shows sections in, left to right and then down. Applied here rather than
 * in the service so readings already saved in the history follow it too. Unknown sections go last.
 */
const ORDER = ['model', 'ids', 'software', 'battery', 'storage', 'cellular'];
const rank = (id: string) => {
  const i = ORDER.indexOf(id);
  return i === -1 ? ORDER.length : i;
};

/** Everything the phone reported, except the technical values, which have the Advanced tab. */
export function DeviceReadout({ reading }: { reading: DeviceReading }) {
  const sections = reading.sections.filter(s => !s.advanced).sort((a, b) => rank(a.id) - rank(b.id));
  if (sections.length === 0) return null;

  return (
    <div className="grid gap-4 lg:grid-cols-2 xl:grid-cols-3">
      {sections.map(section => (
        <Panel key={section.id} title={section.name}>
          <Rows rows={section.rows} />
        </Panel>
      ))}
    </div>
  );
}

/** The technical values and the untouched plist dump, for the Advanced tab. */
export function AdvancedData({ reading }: { reading: DeviceReading }) {
  const advanced = reading.sections.filter(s => s.advanced);
  const raw = JSON.stringify(reading.raw, null, 2);

  return (
    <div className="space-y-4">
      {advanced.map(section => (
        <Panel key={section.id} title={section.name}>
          <dl className="grid gap-x-10 sm:grid-cols-2 xl:grid-cols-3">
            {section.rows.map(row => (
              <div key={row.label} className="flex flex-wrap items-start justify-between gap-x-4 border-b border-slate-100 py-1.5 dark:border-slate-800">
                <dt className="flex-none text-sm text-slate-500 dark:text-slate-400">{row.label}</dt>
                <dd className="ml-auto min-w-0 text-right text-sm font-semibold">
                  <CopyValue value={row.value} label={row.label} />
                </dd>
              </div>
            ))}
          </dl>
        </Panel>
      ))}

      <section className="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm sm:p-5 dark:border-slate-800 dark:bg-slate-900">
        <div className="flex items-center justify-between gap-3">
          <h2 className="text-xs font-bold tracking-[0.08em] text-slate-500 uppercase dark:text-slate-400">Every Raw Value the Phone Reported</h2>
          <CopyAll
            value={raw}
            label="every raw value"
            className="border-slate-200 bg-white text-slate-700 hover:bg-slate-50 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-200 dark:hover:bg-slate-800"
          />
        </div>
        <pre className="scroll-thin mt-3 max-h-[70vh] overflow-auto rounded-lg bg-slate-950 p-3 font-mono text-xs leading-relaxed text-slate-200">{raw}</pre>
      </section>
    </div>
  );
}
