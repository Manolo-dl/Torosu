import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';
import { danColor, ELITE_DAN, textOn } from '@shared/utils';
import { Dan } from '../../osu-reader.model';

const GREEK: Record<string, string> = {
  alpha: 'α', beta: 'β', gamma: 'γ', delta: 'δ', epsilon: 'ε',
  zeta: 'ζ', eta: 'η', theta: 'θ', iota: 'ι', kappa: 'κ',
};

/** What fits in the emblem: the number, the Greek letter, or two letters of a named dan (`azimuth` Az). */
export function danGlyph(label: string): string {
  if (/^\d+$/.test(label)) return label;
  return GREEK[label] ?? label.slice(0, 2).replace(/^./, c => c.toUpperCase());
}

let nextId = 0;

/**
 * A dan as an emblem: a hexagon for rice, a long note for LN, coloured by how
 * hard it is, with its number or letter inside and its `+`/`-` as marks beside.
 */
@Component({
  selector: 'app-dan-emblem',
  template: `
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

      @if (dan().family === 'rice') {
        <path [attr.d]="hexagon" [attr.fill]="'url(#' + id + 'fill)'" />
        <path [attr.d]="hexagon" [attr.fill]="'url(#' + id + 'shade)'" />
        <path
          [attr.d]="hexagon"
          fill="none"
          [attr.stroke]="rim()"
          [attr.stroke-width]="elite() ? 2.5 : 1.5"
          [attr.stroke-opacity]="elite() ? 1 : 0.45"
          stroke-linejoin="round"
        />
      } @else {
        <rect x="5" y="1" width="30" height="42" rx="9" [attr.fill]="'url(#' + id + 'fill)'" />
        <rect x="5" y="1" width="30" height="42" rx="9" [attr.fill]="'url(#' + id + 'shade)'" />
        <!-- The head of the long note. -->
        <rect x="5" y="31" width="30" height="12" rx="6" fill="#fff" fill-opacity="0.28" />
        <rect
          x="5" y="1" width="30" height="42" rx="9"
          fill="none"
          [attr.stroke]="rim()"
          [attr.stroke-width]="elite() ? 2.5 : 1.5"
          [attr.stroke-opacity]="elite() ? 1 : 0.45"
        />
      }

      <text
        x="20"
        [attr.y]="dan().family === 'rice' ? 23 : 18"
        text-anchor="middle"
        dominant-baseline="central"
        [attr.fill]="text()"
        [attr.font-size]="glyph().length > 1 ? 15 : 20"
        font-weight="800"
      >
        {{ glyph() }}
      </text>

      @for (mark of marks(); track $index) {
        <path [attr.d]="mark" [attr.fill]="markColor()" stroke="#000" stroke-opacity="0.3" stroke-width="0.75" stroke-linejoin="round" />
      }
    </svg>
  `,
  styles: `
    :host {
      display: inline-flex;
      flex-shrink: 0;
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

  protected readonly id = `dan-emblem-${nextId++}-`;
  /** Pointy-top hexagon, 40 x 44. */
  protected readonly hexagon = 'M20 1.5 L37.5 11.5 V32.5 L20 42.5 L2.5 32.5 V11.5 Z';

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
}
