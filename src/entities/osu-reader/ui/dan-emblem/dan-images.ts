// The dan badges of Mania Tracker (aleju03/mania-hub, `src/lib/dan-images.ts`),
// used with its author's permission; the art is in `assets/images/dans`, see
// THIRD-PARTY-NOTICES.md. 4K rice: `reform` (svg 1-10, webp alpha to kappa);
// 4K LN: `ln` (1-17); 7K: circles, and `ln-` diamonds for LN (0-10, gamma,
// azimuth, zenith, stellium); 6K: hexagons, and `ln-` ones for LN (0-9,
// terra, celestial, mystery, nihility, finish).

const BASE = 'assets/images/dans';

const REFORM_EXTENSIONS: Record<string, 'svg' | 'webp'> = {
  '1': 'svg', '2': 'svg', '3': 'svg', '4': 'svg', '5': 'svg',
  '6': 'svg', '7': 'svg', '8': 'svg', '9': 'svg', '10': 'svg',
  alpha: 'webp', beta: 'webp', gamma: 'webp', delta: 'webp', epsilon: 'webp',
  zeta: 'webp', eta: 'webp', theta: 'webp', iota: 'webp', kappa: 'webp',
};

const SEVEN_K = new Set(['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '10', 'gamma', 'azimuth', 'zenith', 'stellium']);
const SIX_K = new Set(['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'terra', 'celestial', 'mystery', 'nihility', 'finish']);

/** Each badge has a drawing per variant, with its marks painted in: `7+` is `7-mid-high`. */
const VARIANT_FILES: Record<string, string> = {
  '--': 'low',
  '-': 'mid-low',
  '+': 'mid-high',
  '++': 'high',
};

/** The badge of a dan without its variant, or `null` when there is none (other keymodes). */
function badge(label: string, family: 'rice' | 'ln', keyCount: number | null): string | null {
  label = label.toLowerCase();
  if (keyCount === 7) {
    if (!SEVEN_K.has(label)) return null;
    return `${BASE}/7k/${family === 'ln' ? 'ln-' : ''}${label}.svg`;
  }
  if (keyCount === 6) {
    if (!SIX_K.has(label)) return null;
    return `${BASE}/6k/${family === 'ln' ? 'ln-' : ''}${label}.svg`;
  }
  // The 4K ones would be wrong on the other keymodes, which have their own courses.
  if (keyCount !== null && keyCount !== 4) return null;
  if (family === 'ln' && /^(1[0-7]|[1-9])$/.test(label)) return `${BASE}/ln/${label}.svg`;
  const extension = REFORM_EXTENSIONS[label];
  return extension ? `${BASE}/reform/${label}.${extension}` : null;
}

/** The badge of a dan with its variant drawn in, e.g. `7+` on 4K: `assets/images/dans/reform/7-mid-high.svg`. */
export function danImage(label: string, variant: string | null, family: 'rice' | 'ln', keyCount: number | null): string | null {
  const src = badge(label, family, keyCount);
  const file = variant ? VARIANT_FILES[variant] : undefined;
  return src && file ? src.replace(/\.(svg|webp)$/, `-${file}.$1`) : src;
}
