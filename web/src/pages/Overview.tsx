import { useEffect } from 'react';
import { Link, useNavigate } from 'react-router';
import { CopyAll } from '@/components/Copy';
import { DeviceThumb } from '@/components/DeviceThumb';
import { UsbIcon } from '@/components/icons';
import { useStore } from '@/lib/store';
import { useHead } from '@/lib/use-head';

let autoOpened = false;

export default function Overview() {
  const { devices, loaded, serviceOffline, toolError } = useStore();
  useHead({});

  // Opened from the tray with one phone plugged in, go straight to it. Only on the first visit: once
  // someone has come back here on purpose, this page stays put.
  const navigate = useNavigate();
  useEffect(() => {
    if (!loaded || autoOpened) return;
    autoOpened = true;
    const only = devices.length === 1 ? devices[0] : undefined;
    if (only) navigate(`/device/${only.udid}`, { replace: true });
  }, [loaded, devices, navigate]);

  if (loaded && devices.length === 0) {
    const setup = !serviceOffline && toolError !== null;
    return (
      <section className="mx-auto max-w-lg rounded-2xl border border-dashed border-slate-300 bg-white p-6 text-center sm:p-8 dark:border-slate-700 dark:bg-slate-900">
        <UsbIcon className="mx-auto size-8 text-slate-300 dark:text-slate-600" />
        <h1 className="mt-3 text-lg font-extrabold tracking-tight">
          {serviceOffline ? 'iPhone Inspector Isn’t Running' : setup ? 'One More Step to Read iPhones' : 'No iPhone Connected'}
        </h1>
        <p className="mx-auto mt-1 max-w-sm text-sm leading-relaxed text-slate-500 dark:text-slate-400">
          {serviceOffline
            ? 'Launch the app again, then reload this page.'
            : setup
              ? toolError
              : 'Plug an iPhone in with a cable and unlock it. Its stats open here as soon as it’s trusted.'}
        </p>
        {setup && <InstallHint />}
      </section>
    );
  }

  return (
    <>
      <h1 className="text-2xl font-extrabold tracking-tight">Connected iPhones</h1>
      <ul className="mt-5 grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
        {devices.map(device => (
          <li key={device.udid}>
            <Link
              to={`/device/${device.udid}`}
              className="flex items-center gap-4 rounded-2xl border border-slate-200 bg-white p-4 shadow-sm transition hover:border-slate-300 hover:shadow dark:border-slate-800 dark:bg-slate-900 dark:hover:border-slate-700">
              <span className="flex-none rounded-xl bg-slate-100 p-2 dark:bg-slate-800/60">
                <DeviceThumb device={device} className="h-16 w-auto" />
              </span>
              <span className="min-w-0">
                <span className="block truncate text-lg font-bold">{device.model ?? device.name}</span>
                <span className="block truncate text-sm text-slate-500 dark:text-slate-400">
                  {device.paired ? [device.name, device.ios && `iOS ${device.ios}`].filter(Boolean).join(' · ') : 'Tap Trust on the Phone'}
                </span>
              </span>
            </Link>
          </li>
        ))}
      </ul>
    </>
  );
}

/**
 * What the phone is reached through is part of the system: built into macOS, Apple's USB driver on
 * Windows, and usbmuxd on Linux. Only the last is an install command.
 */
function InstallHint() {
  const ua = navigator.userAgent;
  const command = /Linux/.test(ua) ? 'sudo apt install usbmuxd' : null;
  if (!command) {
    return /Windows/.test(ua) ? (
      <p className="mx-auto mt-3 max-w-sm text-sm leading-relaxed text-slate-500 dark:text-slate-400">
        Get the Apple Devices app from the Microsoft Store, then restart iPhone Inspector.
      </p>
    ) : null;
  }
  return (
    <div className="mx-auto mt-4 flex max-w-sm items-center justify-between gap-2 rounded-lg bg-slate-950 py-1.5 pr-1.5 pl-3 text-left">
      <code className="min-w-0 font-mono text-xs break-words text-slate-200">{command}</code>
      <CopyAll value={command} label="the install command" className="flex-none border-slate-700 bg-slate-900 text-slate-200 hover:bg-slate-800" />
    </div>
  );
}
