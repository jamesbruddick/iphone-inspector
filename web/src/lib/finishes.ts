/**
 * Apple finishes and rear-camera layouts, for the illustration.
 *
 * Hex values are close approximations of each finish, not official Apple values. The table itself
 * lives in `finishes.json` so the Rust service, which matches a reported color to the nearest
 * finish, reads exactly the same one.
 */

import table from './finishes.json';

export interface Finish {
  name: string;
  hex: string;
}

const FINISHES: Record<string, Finish[]> = Object.fromEntries(
  Object.entries(table as unknown as Record<string, [string, string][]>).map(([model, list]) => [model, list.map(([name, hex]) => ({ name, hex }))]),
);

export type CameraLayout =
  | 'single'
  | 'pill-horizontal'
  | 'pill-flash-inside'
  | 'pill-dual'
  | 'square-vertical'
  | 'square-diagonal'
  | 'square-triple'
  | 'square-triple-lidar'
  | 'plateau-single'
  | 'plateau-dual'
  | 'plateau-triple';

/** Rear-camera arrangement by model. Every model in `finishes.json` must appear here exactly once. */
export const LAYOUTS: Record<CameraLayout, string[]> = {
  single: ['iPhone 8', 'iPhone SE (2nd generation)', 'iPhone SE (3rd generation)', 'iPhone XR', 'iPhone 16e', 'iPhone 17e'],
  'pill-horizontal': ['iPhone 8 Plus'],
  'pill-flash-inside': ['iPhone X', 'iPhone XS', 'iPhone XS Max'],
  'pill-dual': ['iPhone 16', 'iPhone 16 Plus', 'iPhone 17'],
  'square-vertical': ['iPhone 11', 'iPhone 12', 'iPhone 12 mini'],
  'square-diagonal': ['iPhone 13', 'iPhone 13 mini', 'iPhone 14', 'iPhone 14 Plus', 'iPhone 15', 'iPhone 15 Plus'],
  'square-triple': ['iPhone 11 Pro', 'iPhone 11 Pro Max'],
  'square-triple-lidar': [
    'iPhone 12 Pro',
    'iPhone 12 Pro Max',
    'iPhone 13 Pro',
    'iPhone 13 Pro Max',
    'iPhone 14 Pro',
    'iPhone 14 Pro Max',
    'iPhone 15 Pro',
    'iPhone 15 Pro Max',
    'iPhone 16 Pro',
    'iPhone 16 Pro Max',
  ],
  'plateau-single': ['iPhone Air'],
  'plateau-dual': ['iPhone Duo'],
  'plateau-triple': ['iPhone 17 Pro', 'iPhone 17 Pro Max', 'iPhone 18 Pro', 'iPhone 18 Pro Max'],
};

const LAYOUT_BY_MODEL: Record<string, CameraLayout> = {};
for (const [layout, names] of Object.entries(LAYOUTS)) {
  for (const n of names) LAYOUT_BY_MODEL[n] = layout as CameraLayout;
}

/**
 * Where the Apple logo sits on the back, as a fraction of the height from the top. It has moved
 * over the years: high on the iPhone 8, a little above center through the XR, dead center from the
 * iPhone 11 on (and the SE models that followed), and lower on the 17 Pro and 18 Pro, where it is
 * centered in the glass panel below the camera plateau.
 */
const LOGO_Y: Record<string, number> = {
  'iPhone 8': 0.36,
  'iPhone 8 Plus': 0.36,
  'iPhone X': 0.43,
  'iPhone XS': 0.43,
  'iPhone XS Max': 0.43,
  'iPhone XR': 0.43,
  'iPhone 17 Pro': 0.62,
  'iPhone 17 Pro Max': 0.62,
  'iPhone 18 Pro': 0.62,
  'iPhone 18 Pro Max': 0.62,
};

export const logoYFor = (name: string): number => LOGO_Y[name] ?? 0.5;

export const finishesFor = (name: string): Finish[] => FINISHES[name] ?? [];
export const layoutFor = (name: string): CameraLayout => LAYOUT_BY_MODEL[name] ?? 'square-diagonal';

export const rgb = (hex: string): [number, number, number] => [1, 3, 5].map(i => Number.parseInt(hex.slice(i, i + 2), 16)) as [number, number, number];

export function mix(hex: string, to: string, amount: number): string {
  const a = rgb(hex);
  const b = rgb(to);
  return `#${a
    .map((v, i) =>
      Math.round(v + (b[i]! - v) * amount)
        .toString(16)
        .padStart(2, '0'),
    )
    .join('')}`;
}

export function isLight(hex: string): boolean {
  const [r, g, b] = rgb(hex);
  return 0.2126 * r + 0.7152 * g + 0.0722 * b > 150;
}
