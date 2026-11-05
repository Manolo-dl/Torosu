import { ChangeDetectionStrategy, Component, computed, effect, input, signal } from '@angular/core';
import { danColor, ELITE_DAN, textOn } from '@shared/utils';
import { Dan } from '../../osu-reader.model';
import { danImage } from './dan-images';

const SYMBOLS: Record<string, string> = {
  // 4K, after 10.
  alpha: 'α', beta: 'β', gamma: 'γ', delta: 'δ', epsilon: 'ε',
  zeta: 'ζ', eta: 'η', theta: 'θ', iota: 'ι', kappa: 'κ',
  // 7K, after 10 and gamma: the symbols of its courses.
  azimuth: 'ψ', zenith: 'ζ', stellium: '★',
};

/** What fits in the emblem: the number, the symbol, or two letters of a named dan (`terra` Te). */
export function danGlyph(label: string): string {
  if (/^\d+$/.test(label)) return label;
  return SYMBOLS[label] ?? label.slice(0, 2).replace(/^./, c => c.toUpperCase());
}

type Shape = 'hexagon' | 'capsule' | 'circle' | 'diamond';

/** The outlines, in a 40 x 44 box. */
const OUTLINES: Record<Shape, string> = {
  hexagon: 'M20 1.5 L37.5 11.5 V32.5 L20 42.5 L2.5 32.5 V11.5 Z',
  // A long note.
  capsule: 'M14 1 H26 A9 9 0 0 1 35 10 V34 A9 9 0 0 1 26 43 H14 A9 9 0 0 1 5 34 V10 A9 9 0 0 1 14 1 Z',
  circle: 'M0.5 22 A19.5 19.5 0 1 1 39.5 22 A19.5 19.5 0 1 1 0.5 22 Z',
  diamond: 'M20 1.5 L38.5 22 L20 42.5 L1.5 22 Z',
};

let nextId = 0;

/**
 * A dan as its course badge, Mania Tracker's art (see `dan-images.ts`). For a
 * dan without one, an emblem drawn here, coloured by how hard it is, with its number or symbol
 * inside and its `+`/`-` as marks beside. 7K dans are circles (rice) and
 * diamonds (LN), like its courses; the others hexagons (rice) and long notes
 * (LN).
 */
@Component({
  selector: 'app-dan-emblem',
  template: `
    @if (image(); as src) {
      <!-- The drawings with marks are wider: the height is set, the width follows. -->
      <img alt="" [src]="src" [style.height.px]="size()" [attr.title]="dan().name" (error)="failed.set(true)" />
    } @else {
    <svg
      xmlns="http://www.w3.org/2000/svg"
      role="img"
      [attr.viewBox]="'0 0 ' + width() + ' 44'"
      [attr.width]="(size() * width()) / 44"
      [attr.height]="size()"
      [attr.aria-label]="dan().name"
    >
      <defs>
        <linearGradient [attr.id]="id + 'fill'" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" [attr.stop-color]="color()" stop-opacity="0.78" />
          <stop offset="0.45" [attr.stop-color]="color()" />
          <stop offset="1" [attr.stop-color]="color()" />
        </linearGradient>
        <linearGradient [attr.id]="id + 'shade'" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stop-color="#fff" stop-opacity="0.35" />
          <stop offset="0.5" stop-color="#fff" stop-opacity="0" />
          <stop offset="1" stop-color="#000" stop-opacity="0.22" />
        </linearGradient>
      </defs>

      <path [attr.d]="outline()" [attr.fill]="'url(#' + id + 'fill)'" />
      <path [attr.d]="outline()" [attr.fill]="'url(#' + id + 'shade)'" />
      @if (shape() === 'capsule') {
        <!-- The head of the long note. -->
        <rect x="5" y="31" width="30" height="12" rx="6" fill="#fff" fill-opacity="0.28" />
      }
      <path
        [attr.d]="outline()"
        fill="none"
        [attr.stroke]="rim()"
        [attr.stroke-width]="elite() ? 2.5 : 1.5"
        [attr.stroke-opacity]="elite() ? 1 : 0.45"
        stroke-linejoin="round"
      />

      <text
        x="20"
        [attr.y]="glyphY()"
        text-anchor="middle"
        dominant-baseline="central"
        [attr.fill]="text()"
        [attr.font-size]="glyphSize()"
        font-weight="800"
      >
        {{ glyph() }}
      </text>

      @for (mark of marks(); track $index) {
        <path [attr.d]="mark" [attr.fill]="markColor()" stroke="#000" stroke-opacity="0.3" stroke-width="0.75" stroke-linejoin="round" />
      }
    </svg>
    }
  `,
  styles: `
    :host {
      display: inline-flex;
      flex-shrink: 0;
    }

    img {
      inline-size: auto;
      filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.35));
    }

    svg {
      overflow: visible;
      filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.35));
    }

    text {
      font-family: inherit;
      letter-spacing: -0.02em;
    }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DanEmblemComponent {
  readonly dan = input.required<Dan>();
  /** Height in px. */
  readonly size = input(40);
  /** Of the map: 7K dans have their own shapes. */
  readonly keyCount = input<number | null>(null);

  protected readonly id = `dan-emblem-${nextId++}-`;

  /** Whether the badge didn't load: the emblem is drawn instead. */
  readonly failed = signal(false);
  readonly image = computed(() => {
    const dan = this.dan();
    return this.failed() ? null : danImage(dan.label, dan.variant, dan.family, this.keyCount());
  });

  readonly shape = computed((): Shape => {
    const ln = this.dan().family === 'ln';
    if (this.keyCount() === 7) return ln ? 'diamond' : 'circle';
    return ln ? 'capsule' : 'hexagon';
  });
  readonly outline = computed(() => OUTLINES[this.shape()]);
  /** Above the head of the long note, otherwise in the middle. */
  readonly glyphY = computed(() => ({ hexagon: 23, capsule: 18, circle: 22, diamond: 22 })[this.shape()]);
  /** The diamond is narrower inside. */
  readonly glyphSize = computed(() => {
    const long = this.glyph().length > 1;
    return this.shape() === 'diamond' ? (long ? 12 : 17) : long ? 15 : 20;
  });

  readonly color = computed(() => danColor(this.dan().value));
  readonly elite = computed(() => this.dan().value >= ELITE_DAN);
  readonly rim = computed(() => (this.elite() ? '#e9c46a' : '#ffffff'));
  readonly text = computed(() => (this.elite() ? '#f5d77a' : textOn(this.color())));
  readonly glyph = computed(() => danGlyph(this.dan().label));
  /** The dark colours of the hardest dans would vanish on a dark background. */
  readonly markColor = computed(() => (this.elite() ? '#e9c46a' : this.color()));

  /** `+` and `++` as triangles up beside the emblem, `-` and `--` as triangles down. */
  readonly marks = computed(() => {
    const variant = this.dan().variant ?? '';
    const up = variant.startsWith('+');
    return Array.from({ length: variant.length }, (_, i) => {
      // Stacked from the middle: one mark centred, two one above the other.
      const y = 22 + (variant.length === 1 ? 0 : i === 0 ? -5 : 5);
      return up ? `M41 ${y + 4} L47 ${y - 4} L53 ${y + 4} Z` : `M41 ${y - 4} L47 ${y + 4} L53 ${y - 4} Z`;
    });
  });
  readonly width = computed(() => (this.marks().length > 0 ? 54 : 40));

  constructor() {
    // Another dan gets its own try.
    effect(() => {
      this.dan();
      this.keyCount();
      this.failed.set(false);
    });
  }
}
