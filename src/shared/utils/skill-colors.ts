import { blendSpectrum } from './osu-colors';

type Spectrum = [number, [number, number, number]][];

/**
 * Colours of the dans by their value (Mania Tracker's raw dan: 1 to 10, then
 * alpha 11, beta 12 ...): cold to warm through the numbered ones, violet for
 * the first Greek ones, dark from delta on, where the emblem gets a gold rim.
 */
const DAN_SPECTRUM: Spectrum = [
  [0, [0x8f, 0x9b, 0xb3]],
  [1, [0x5a, 0xa9, 0xff]],
  [3, [0x3c, 0xcf, 0xbf]],
  [4.5, [0x5c, 0xcf, 0x6c]],
  [6, [0xf0, 0xc4, 0x3c]],
  [7.5, [0xf2, 0x8c, 0x3a]],
  [9, [0xe8, 0x4a, 0x4a]],
  [10, [0xd3, 0x2f, 0x6e]],
  [11, [0xae, 0x3e, 0xc9]],
  [13, [0x70, 0x48, 0xe8]],
  [15, [0x3b, 0x23, 0x80]],
  [17, [0x16, 0x0e, 0x30]],
];

/** From here on a dan is one of the hardest: gold rim and text. */
export const ELITE_DAN = 14;

/** Colours of the MSD, on Etterna's scale: blue for easy, red at 30, violet to black above. */
const MSD_SPECTRUM: Spectrum = [
  [5, [0x8f, 0x9b, 0xb3]],
  [10, [0x5a, 0xa9, 0xff]],
  [15, [0x3c, 0xcf, 0xbf]],
  [19, [0x5c, 0xcf, 0x6c]],
  [23, [0xf0, 0xc4, 0x3c]],
  [27, [0xf2, 0x8c, 0x3a]],
  [30, [0xe8, 0x4a, 0x4a]],
  [33, [0xd3, 0x2f, 0x6e]],
  [36, [0xae, 0x3e, 0xc9]],
  [40, [0x70, 0x48, 0xe8]],
  [45, [0x2a, 0x1a, 0x5e]],
];

export function danColor(value: number): string {
  return blendSpectrum(DAN_SPECTRUM, value);
}

export function msdColor(value: number): string {
  return blendSpectrum(MSD_SPECTRUM, value);
}

/** Dark or white text, whichever reads better on `background` (`#rrggbb`). */
export function textOn(background: string): string {
  const [r, g, b] = [1, 3, 5].map(i => parseInt(background.slice(i, i + 2), 16) / 255);
  const linear = (c: number) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
  const luminance = 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b);
  // Where black and white have the same contrast.
  return luminance > 0.179 ? 'rgba(0, 0, 0, 0.82)' : '#ffffff';
}
