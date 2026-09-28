import { useId } from 'react';

/**
 * The current charge, drawn the way iOS draws it in the status bar: the percentage, then a battery
 * whose fill is the level. Green with a bolt while charging, red at 20% and below, otherwise the
 * text color - as on the phone itself.
 */
export function BatteryStatus({ level, charging }: { level: number; charging: boolean | null }) {
  // Unique per instance: the page renders this twice (phone and desktop layouts), and a shared id
  // would point both at whichever copy comes first - the hidden one, on a phone.
  const clipId = useId();
  const pct = Math.max(0, Math.min(100, Math.round(level)));
  const low = pct <= 20 && !charging;
  const fill = charging ? '#34C759' : low ? '#FF3B30' : 'currentColor';
  const label = `Battery ${pct}%${charging ? ', charging' : ''}`;

  // Body 26 x 13 with a 1.5px cap, as in iOS. The fill is clipped to the body's rounded shape so a
  // low charge ends in a straight edge, not a pill.
  const bodyW = 26;
  const bodyH = 13;
  return (
    <span className="inline-flex items-center gap-1.5 text-slate-900 dark:text-white" role="img" aria-label={label} title={label}>
      <span className="text-base font-semibold tabular-nums">{pct}%</span>
      <svg viewBox="0 0 29 13" className="h-[13px] w-[29px]" aria-hidden="true">
        <defs>
          <clipPath id={clipId}>
            <rect x="0" y="0" width={bodyW} height={bodyH} rx="4" />
          </clipPath>
        </defs>
        <g clipPath={`url(#${clipId})`}>
          <rect x="0" y="0" width={bodyW} height={bodyH} fill="currentColor" opacity="0.28" />
          <rect x="0" y="0" width={(bodyW * pct) / 100} height={bodyH} fill={fill} />
        </g>
        <path d="M27.2 4.3 a1.6 1.6 0 0 1 1.6 1.6 v1.2 a1.6 1.6 0 0 1 -1.6 1.6 z" fill="currentColor" opacity="0.4" />
        {charging && (
          <path d="M14.2 1.6 L8.6 7.4 H12.4 L11.4 11.4 L17.2 5.6 H13.4 Z" fill="#ffffff" stroke="#1f7a37" strokeWidth="0.4" strokeLinejoin="round" />
        )}
      </svg>
      {charging && <span className="sr-only">Charging</span>}
    </span>
  );
}
