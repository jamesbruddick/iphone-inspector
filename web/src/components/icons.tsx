/**
 * The icon set. Drawn here rather than pulled from a library so every glyph shares one 1.75 stroke
 * and one 16-unit grid, and so the app ships no icon font.
 */

import type { SVGProps } from 'react';

const base = {
  viewBox: '0 0 16 16',
  fill: 'none',
  stroke: 'currentColor',
  strokeWidth: 1.75,
  strokeLinecap: 'round',
  strokeLinejoin: 'round',
  'aria-hidden': true,
} as const;

type IconProps = SVGProps<SVGSVGElement>;

export const CheckIcon = (p: IconProps) => (
  <svg {...base} {...p}>
    <path d="M3 8.5l3.2 3.2L13 5" />
  </svg>
);

/**
 * A ring broken at the top right, with a solid arrowhead on the end of the arc.
 *
 * The usual open corner tacked onto an arc collapses into a stray check mark at 16px - the old glyph read
 * as a lowercase "t" in a circle. A filled head on the arc's own tangent keeps its direction
 * readable at button size, and it is the one place in this set that fills rather than strokes.
 */
export const RefreshIcon = (p: IconProps) => (
  <svg {...base} {...p}>
    <path d="M5.04 4.48A4.6 4.6 0 1 0 10.64 4.23" />
    <path d="M11.99 5.18L8.61 4.77 10.45 2.15Z" fill="currentColor" stroke="none" />
  </svg>
);

export const CopyIcon = (p: IconProps) => (
  <svg {...base} {...p}>
    <rect x="5.75" y="5.75" width="8" height="8" rx="1.75" />
    <path d="M10.25 3.4A1.75 1.75 0 008.5 2h-4.5A2 2 0 002 4v4.5c0 .83.57 1.53 1.35 1.72" />
  </svg>
);

export const AlertIcon = (p: IconProps) => (
  <svg {...base} {...p}>
    <circle cx="8" cy="8" r="6.25" />
    <path d="M8 4.75v3.75M8 11.1v.05" />
  </svg>
);

export const MenuIcon = (p: IconProps) => (
  <svg {...base} {...p}>
    <path d="M2.5 4.5h11M2.5 8h11M2.5 11.5h11" />
  </svg>
);

export const CloseIcon = (p: IconProps) => (
  <svg {...base} {...p}>
    <path d="M4 4l8 8M12 4l-8 8" />
  </svg>
);

export const UsbIcon = (p: IconProps) => (
  <svg {...base} {...p}>
    <path d="M8 14V3.5M8 3.5L6 5.5M8 3.5l2 2M5.5 8.5v2a1 1 0 001 1H8M10.5 7.5v3" />
    <circle cx="10.5" cy="6.25" r="1.1" />
  </svg>
);

export const LockIcon = (p: IconProps) => (
  <svg {...base} {...p}>
    <rect x="3.5" y="7" width="9" height="6.5" rx="1.5" />
    <path d="M5.75 7V5.25a2.25 2.25 0 014.5 0V7" />
  </svg>
);

export const ClockIcon = (p: IconProps) => (
  <svg {...base} {...p}>
    <circle cx="8" cy="8" r="6.25" />
    <path d="M8 4.75V8l2.25 1.5" />
  </svg>
);
