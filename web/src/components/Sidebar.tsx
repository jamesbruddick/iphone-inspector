import { NavLink } from 'react-router';
import { DeviceThumb } from '@/components/DeviceThumb';
import { ClockIcon, UsbIcon } from '@/components/icons';
import { RemoveButton } from '@/components/RemoveButton';
import { whenRead } from '@/lib/dates';
import { useStore } from '@/lib/store';

const rowClass = ({ isActive }: { isActive: boolean }) =>
  `flex min-w-0 flex-1 items-center gap-2.5 rounded-xl border p-2.5 pl-3 transition ${
    isActive
      ? 'border-slate-300 bg-white shadow-sm dark:border-slate-700 dark:bg-slate-800'
      : 'border-transparent hover:bg-slate-200/50 dark:hover:bg-slate-800/60'
  }`;

function Heading({ icon, children, count }: { icon: React.ReactNode; children: React.ReactNode; count: number }) {
  return (
    <h2 className="flex items-center gap-1.5 px-1 text-xs font-bold tracking-[0.08em] text-slate-500 uppercase dark:text-slate-400">
      {icon}
      {children}
      {count > 0 && <span className="font-semibold text-slate-400 dark:text-slate-500">{count}</span>}
    </h2>
  );
}

export function Sidebar({ onNavigate }: { onNavigate?: () => void }) {
  const { devices, loaded, serviceOffline, toolError, history, removeFromHistory } = useStore();

  // A phone that is plugged in right now is listed once, under Connected.
  const connected = new Set(devices.map(d => d.udid));
  const past = history.filter(h => !connected.has(h.udid));

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="scroll-thin min-h-0 flex-1 space-y-5 overflow-y-auto pb-2">
        <section>
          <Heading icon={<UsbIcon className="size-3.5" />} count={devices.length}>
            Connected
          </Heading>
          {loaded && devices.length === 0 && !serviceOffline && (
            <p className="mt-2 px-1 text-sm leading-relaxed text-slate-500 dark:text-slate-400">
              Plug in an iPhone and unlock it. It shows up here within a few seconds.
            </p>
          )}
          <ul className="mt-2 space-y-1">
            {devices.map(device => (
              <li key={device.udid} className="flex">
                <NavLink to={`/device/${device.udid}`} onClick={onNavigate} className={rowClass}>
                  <span className="flex-none rounded-lg bg-slate-200/70 p-1 dark:bg-slate-700/40">
                    <DeviceThumb device={device} className="h-7 w-auto" />
                  </span>
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-base font-semibold text-slate-900 dark:text-slate-100">{device.model ?? device.name}</span>
                    <span className="block truncate text-xs text-slate-500 dark:text-slate-400">
                      {device.paired ? [device.name, device.ios && `iOS ${device.ios}`].filter(Boolean).join(' · ') : 'Tap Trust on the Phone'}
                    </span>
                  </span>
                </NavLink>
              </li>
            ))}
          </ul>
        </section>

        {past.length > 0 && (
          <section>
            <Heading icon={<ClockIcon className="size-3.5" />} count={past.length}>
              History
            </Heading>
            <ul className="mt-2 space-y-1">
              {past.map(item => (
                <li key={item.udid} className="flex items-center gap-1">
                  <NavLink to={`/device/${item.udid}`} onClick={onNavigate} className={rowClass}>
                    <span className="flex-none rounded-lg bg-slate-200/70 p-1 opacity-80 dark:bg-slate-700/40">
                      <DeviceThumb device={item} className="h-7 w-auto" />
                    </span>
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-base font-semibold text-slate-700 dark:text-slate-300">{item.model ?? 'iPhone'}</span>
                      <span className="block truncate text-xs text-slate-500 dark:text-slate-400">
                        {[item.deviceName, whenRead(item.savedAt)].filter(Boolean).join(' · ')}
                      </span>
                    </span>
                  </NavLink>
                  <RemoveButton label={item.deviceName ?? item.model ?? 'this iPhone'} onRemove={() => removeFromHistory(item.udid)} />
                </li>
              ))}
            </ul>
          </section>
        )}
      </div>

      {(serviceOffline || toolError) && (
        <p className="rounded-lg border border-amber-200 bg-amber-50 px-2.5 py-2 text-xs leading-relaxed text-amber-800 dark:border-amber-900/60 dark:bg-amber-950/40 dark:text-amber-300">
          {serviceOffline ? 'iPhone Inspector is not running. Launch the app again to reconnect.' : toolError}
        </p>
      )}
    </div>
  );
}
