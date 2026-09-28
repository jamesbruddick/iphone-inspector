/**
 * A drawing of the back of the phone in a given finish, with the right camera arrangement.
 *
 * Deliberately an illustration rather than an Apple product photo: it ships with the app, needs no
 * network, and is drawn from simple shapes. It is a reference for the finish and the camera
 * layout, not a picture of the actual handset in front of you.
 */

import { isLight, layoutFor, logoYFor, mix } from '@/lib/finishes';

const W = 100;
const H = 200;
const DEFAULT_BODY = '#C7CCD1';

/**
 * The Apple logo, in a 20 x 24 box with its origin at the top left: the leaf, then the body with the
 * bite out of its right side.
 */
const LOGO =
  'M13.5 1C13.6 3 12.2 5 10.2 5.3 10 3.3 11.6 1.3 13.5 1Z' +
  'M10 6.6C11.2 6.6 12.4 5.8 14 5.8 15.6 5.8 17.2 6.6 18.2 8.1 15.6 9.6 15.9 13.6 18.8 14.8 18.1 16.9 17.2 18.7 15.9 20.3 14.9 21.6 13.9 22.6 12.5 22.6 11.1 22.6 10.7 21.8 9.1 21.8 7.5 21.8 7 22.6 5.6 22.6 4.2 22.6 3.1 21.4 2.1 20 0.3 17.4-0.6 13.2 1 10.3 1.9 8.6 3.6 7.2 5.5 7.2 7.2 7.2 8.8 6.6 10 6.6Z';
const LOGO_W = 16;

export function PhoneArt({ model, hex, label, className = '' }: { model: string; hex: string | null; label?: string; className?: string }) {
  const layout = layoutFor(model);
  const body = hex && /^#[0-9a-f]{6}$/i.test(hex) ? hex : DEFAULT_BODY;
  const light = isLight(body);
  // The metal band around the back glass: a shade off the finish, the way the frame reads beside the
  // glass on the real thing.
  const frame = light ? mix(body, '#000000', 0.13) : mix(body, '#FFFFFF', 0.16);
  const logo = light ? mix(body, '#000000', 0.2) : mix(body, '#FFFFFF', 0.18);
  const edge = mix(body, '#000000', light ? 0.18 : 0.35);
  const bump = light ? mix(body, '#000000', 0.07) : mix(body, '#FFFFFF', 0.1);
  const bumpEdge = mix(body, '#000000', light ? 0.14 : 0.3);

  const parts: React.ReactNode[] = [];
  let key = 0;

  const lens = (x: number, y: number, r = 7) => {
    parts.push(<circle key={key++} cx={x} cy={y} r={r + 1.2} fill={bumpEdge} />);
    parts.push(<circle key={key++} cx={x} cy={y} r={r} fill="#1B1C1F" />);
    parts.push(<circle key={key++} cx={x} cy={y} r={r * 0.45} fill="#2E3440" />);
    parts.push(<circle key={key++} cx={x - r * 0.25} cy={y - r * 0.25} r={r * 0.14} fill="#8FA3BF" />);
  };
  // The LiDAR scanner: a smaller, flat black disc with no glass highlight, which is how it reads on
  // the real hardware next to the lenses.
  const lidar = (x: number, y: number) => {
    parts.push(<circle key={key++} cx={x} cy={y} r={4.4} fill={bumpEdge} />);
    parts.push(<circle key={key++} cx={x} cy={y} r={3.5} fill="#121316" />);
  };
  const mic = (x: number, y: number) => parts.push(<circle key={key++} cx={x} cy={y} r={0.9} fill={bumpEdge} />);
  const flash = (x: number, y: number) => parts.push(<circle key={key++} cx={x} cy={y} r={2.6} fill="#F4EBD0" stroke={bumpEdge} strokeWidth={1} />);
  const plate = (x: number, y: number, w: number, h: number, r: number) =>
    parts.push(<rect key={key++} x={x} y={y} width={w} height={h} rx={r} fill={bump} stroke={bumpEdge} strokeWidth={1} />);

  switch (layout) {
    // iPhone 8, SE (2nd and 3rd gen), XR, 16e, 17e: one lens, flash beside it.
    case 'single':
      lens(18, 18);
      flash(32, 17);
      mic(26, 17);
      break;
    // iPhone 8 Plus: a horizontal pill, flash to its right.
    case 'pill-horizontal':
      plate(8, 8, 42, 21, 10.5);
      lens(18.5, 18.5);
      lens(39.5, 18.5);
      flash(57, 18.5);
      break;
    // iPhone X, XS, XS Max: a vertical pill with the flash between the two lenses.
    case 'pill-flash-inside':
      plate(8, 8, 21, 52, 10.5);
      lens(18.5, 18.5);
      lens(18.5, 49.5);
      flash(18.5, 34);
      break;
    // iPhone 16, 16 Plus, 17: a vertical pill, flash outside it.
    case 'pill-dual':
      plate(8, 8, 21, 42, 10.5);
      lens(18.5, 18.5);
      lens(18.5, 39.5);
      flash(36, 22);
      mic(36, 30);
      break;
    // iPhone 11, 12, 12 mini: square bump, lenses stacked on the left, flash top right.
    case 'square-vertical':
      plate(7, 7, 40, 40, 10);
      lens(18, 18);
      lens(18, 36);
      flash(36, 18);
      mic(36, 36);
      break;
    // iPhone 13 to 15 (and Plus): square bump, lenses on the diagonal.
    case 'square-diagonal':
      plate(7, 7, 40, 40, 10);
      lens(18, 18);
      lens(36, 36);
      flash(36, 17);
      mic(18, 36);
      break;
    // iPhone 11 Pro and Pro Max: three lenses, flash top right, no LiDAR yet.
    case 'square-triple':
      plate(6, 6, 46, 46, 11);
      lens(18, 18, 7.5);
      lens(18, 40, 7.5);
      lens(39, 29, 7.5);
      flash(40, 13);
      mic(40, 45);
      break;
    // iPhone 12 Pro to 16 Pro Max: the same three lenses, with LiDAR below the flash.
    case 'square-triple-lidar':
      plate(6, 6, 46, 46, 11);
      lens(18, 18, 7.5);
      lens(18, 40, 7.5);
      lens(39, 29, 7.5);
      flash(40, 13);
      lidar(40, 45);
      break;
    // iPhone Air: a full-width plateau with one lens, flash beside it.
    case 'plateau-single':
      plate(4, 8, 92, 24, 12);
      lens(18, 20);
      flash(32, 20);
      mic(40, 20);
      break;
    // iPhone Duo: a plateau with two lenses side by side.
    case 'plateau-dual':
      plate(4, 8, 92, 26, 13);
      lens(17, 21);
      lens(37, 21);
      flash(82, 21);
      break;
    // iPhone 17 Pro and 18 Pro (and Pro Max): a full-width plateau, three lenses on the left, and
    // the flash and LiDAR scanner on the right.
    case 'plateau-triple':
      plate(3, 5, 94, 52, 14);
      lens(17, 18, 7.5);
      lens(17, 42, 7.5);
      lens(37, 30, 7.5);
      flash(83, 19);
      mic(83, 30);
      lidar(83, 41);
      break;
  }

  // The 17 Pro and 18 Pro are aluminum on the back, with a separate glass panel below the plateau
  // for wireless charging - its own inset, with a lip around it.
  const glassPanel = layout === 'plateau-triple';
  const panel = light ? mix(body, '#FFFFFF', 0.18) : mix(body, '#FFFFFF', 0.06);

  const scale = LOGO_W / 20;
  const logoX = W / 2 - LOGO_W / 2;
  const logoY = H * logoYFor(model) - (24 * scale) / 2;

  return (
    <svg viewBox={`0 0 ${W} ${H}`} className={className} {...(label ? { role: 'img', 'aria-label': label } : { 'aria-hidden': true })}>
      {/* Frame, then the back inset inside it. */}
      <rect x={0.75} y={0.75} width={W - 1.5} height={H - 1.5} rx={16.5} fill={frame} stroke={edge} strokeWidth={0.75} />
      <rect x={3} y={3} width={W - 6} height={H - 6} rx={14} fill={body} />
      {glassPanel && <rect x={5.5} y={61} width={W - 11} height={H - 67} rx={11} fill={panel} stroke={edge} strokeWidth={0.6} />}
      {parts}
      <path d={LOGO} fill={logo} transform={`translate(${logoX} ${logoY}) scale(${scale})`} />
    </svg>
  );
}
