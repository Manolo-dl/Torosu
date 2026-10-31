/**
 * osu!'s star rating spectrum (the website and lazer): the colour at each of these star values, blended
 * in between. Below 0.1 stars it is grey, from 9 on black.
 */
const STAR_SPECTRUM: [number, [number, number, number]][] = [
  [0.1, [0x42, 0x90, 0xfb]],
  [1.25, [0x4f, 0xc0, 0xff]],
  [2, [0x4f, 0xff, 0xd5]],
  [2.5, [0x7c, 0xff, 0x4f]],
  [3.3, [0xf6, 0xf0, 0x5c]],
  [4.2, [0xff, 0x80, 0x68]],
  [4.9, [0xff, 0x4e, 0x6f]],
  [5.8, [0xc6, 0x45, 0xb8]],
  [6.7, [0x65, 0x63, 0xde]],
  [7.7, [0x18, 0x15, 0x8e]],
  [9, [0x00, 0x00, 0x00]],
];

/** osu! blends the colours with a gamma of 2.2, as d3's `interpolateRgb.gamma(2.2)`. */
const GAMMA = 2.2;

const hex = (rgb: number[]) => '#' + rgb.map(c => Math.round(c).toString(16).padStart(2, '0')).join('');

/** Background colour osu! uses for a star rating. */
export function starBg(stars: number): string {
  if (stars < STAR_SPECTRUM[0][0]) return '#aaaaaa';
  const next = STAR_SPECTRUM.findIndex(([at]) => at > stars);
  if (next === -1) return hex(STAR_SPECTRUM[STAR_SPECTRUM.length - 1][1]);

  const [fromAt, from] = STAR_SPECTRUM[next - 1];
  const [toAt, to] = STAR_SPECTRUM[next];
  const t = (stars - fromAt) / (toAt - fromAt);
  return hex(from.map((a, i) => {
    const [ga, gb] = [a ** GAMMA, to[i] ** GAMMA];
    return (ga + t * (gb - ga)) ** (1 / GAMMA);
  }));
}

/** Text colour osu! uses on top of {@link starBg}: dark, and yellow once the background turns dark. */
export function starFg(stars: number): string {
  return stars >= 6.5 ? '#ffd966' : 'rgba(0, 0, 0, 0.75)';
}
