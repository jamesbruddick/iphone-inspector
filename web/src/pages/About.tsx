import { useHead } from '@/lib/use-head';

function Panel({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-900">
      <h2 className="text-xs font-bold tracking-[0.08em] text-slate-500 uppercase dark:text-slate-400">{title}</h2>
      <div className="mt-3 space-y-2 text-sm leading-relaxed text-slate-600 dark:text-slate-400">{children}</div>
    </section>
  );
}

export default function About() {
  useHead({ title: 'About', description: 'What iPhone Inspector reads from a connected iPhone, and where it goes.' });

  return (
    <>
      <h1 className="text-2xl font-extrabold tracking-tight">About</h1>

      <div className="mt-5 space-y-4">
        <Panel title="What It Reads">
          <p>
            Everything an iPhone reports over USB: model, identifiers, battery health and charge cycles, storage, software, cellular and SIM details, and a
            technical dump of every raw value. Values come straight from the phone and are shown as-is.
          </p>
          <p>
            Battery health is calculated from the raw full-charge and design capacities, so it can differ from Settings by a point or two. Newer iOS versions no
            longer report the housing color, Find My or device management; when a phone stays quiet about something, the page says so rather than guessing.
          </p>
        </Panel>

        <Panel title="Where Your Data Goes">
          <p>
            It stays on this computer. The app runs a small service on 127.0.0.1, which nothing else on your network can reach. Once an iPhone has been read in
            full, its stats are saved to a history file in the app's data folder, so you can look at them again after unplugging it. Remove a phone from the
            history with the × beside it; nothing is ever uploaded.
          </p>
        </Panel>

        <Panel title="The Tray Icon">
          <p>
            iPhone Inspector lives in the menu bar or system tray. Click it to open this page; right-click it to see which iPhones are connected, switch
            starting at login on or off, or quit. When it starts at login it stays in the background and shows a notification when an iPhone is plugged in.
          </p>
        </Panel>
      </div>
    </>
  );
}
