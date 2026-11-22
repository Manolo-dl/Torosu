import { ChangeDetectionStrategy, Component, input } from '@angular/core';

/**
 * The shape of something on its way, a light sweeping over it:
 * `<app-skeleton width="60%" height="0.8rem" />`. `radius` is any length
 * (`pill` for a round one).
 */
@Component({
    selector: 'app-skeleton',
    template: '',
    styles: `
        :host {
            position: relative;
            display: block;
            flex-shrink: 0;
            inline-size: var(--skeleton-width, 100%);
            block-size: var(--skeleton-height, 0.8rem);
            border-radius: var(--skeleton-radius, var(--radius-sm));
            background: color-mix(in oklch, var(--text-1) 8%, transparent);
            overflow: hidden;
        }

        /* Only a transform moves: the compositor does it, nothing is painted again. */
        :host::after {
            content: '';
            position: absolute;
            inset: 0;
            background: linear-gradient(90deg, transparent, color-mix(in oklch, var(--text-1) 9%, transparent), transparent);
            transform: translateX(-100%);
            animation: sweep 1.5s var(--ease-in-out) infinite;
            animation-delay: var(--skeleton-delay, 0s);
        }

        @keyframes sweep {
            to { transform: translateX(100%); }
        }

        :host-context([data-motion='reduced'])::after {
            animation: none;
            opacity: 0;
        }

        @media (prefers-reduced-motion: reduce) {
            :host::after {
                animation: none;
                opacity: 0;
            }
        }
    `,
    host: {
        'aria-hidden': 'true',
        '[style.--skeleton-width]': 'width()',
        '[style.--skeleton-height]': 'height()',
        '[style.--skeleton-radius]': 'radius() === "pill" ? "999px" : radius()',
    },
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SkeletonComponent {
    readonly width = input<string>();
    readonly height = input<string>();
    readonly radius = input<string>();
}
