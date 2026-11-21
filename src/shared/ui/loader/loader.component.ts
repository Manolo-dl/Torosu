import { ChangeDetectionStrategy, Component, input } from '@angular/core';

/**
 * Something is on its way: four lanes of a mania stage lighting up one after
 * another, with what is happening under them: `<app-loader>Analysing…</app-loader>`.
 */
@Component({
    selector: 'app-loader',
    template: `
        <span class="lanes" aria-hidden="true">
            @for (lane of lanes; track lane) {
                <span class="lane" [style.--lane]="lane"></span>
            }
        </span>
        <span class="text"><ng-content /></span>
    `,
    styles: `
        :host {
            --lane-width: 0.3rem;
            --lane-height: 1.25rem;

            display: inline-flex;
            flex-direction: column;
            align-items: center;
            gap: var(--space-3);
            color: var(--text-3);
            font-size: var(--text-sm);
        }

        :host([data-size='s']) { --lane-width: 0.2rem; --lane-height: 0.85rem; }
        :host([data-size='l']) { --lane-width: 0.42rem; --lane-height: 1.9rem; }

        .lanes {
            display: flex;
            align-items: flex-end;
            gap: calc(var(--lane-width) * 0.7);
            block-size: var(--lane-height);
        }

        /* The two inner lanes in the second accent, as on a 4K skin. */
        .lane {
            inline-size: var(--lane-width);
            block-size: 100%;
            border-radius: var(--radius-pill);
            background: var(--accent);
            transform-origin: bottom;
            animation: press 0.9s var(--ease-in-out) infinite;
            animation-delay: calc(var(--lane) * 0.12s);
        }

        .lane:nth-child(2),
        .lane:nth-child(3) {
            background: var(--accent-2);
        }

        @keyframes press {
            0%, 100% { transform: scaleY(0.3); opacity: 0.45; }
            40% { transform: scaleY(1); opacity: 1; }
        }

        .text:empty {
            display: none;
        }

        /* Reduced motion: still, but not frozen at their lowest. */
        :host-context([data-motion='reduced']) .lane {
            animation: none;
            transform: scaleY(0.6);
        }

        @media (prefers-reduced-motion: reduce) {
            .lane {
                animation: none;
                transform: scaleY(0.6);
            }
        }
    `,
    host: { role: 'status', '[attr.data-size]': 'size()' },
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class LoaderComponent {
    readonly size = input<'s' | 'm' | 'l'>('m');
    readonly lanes = [0, 1, 2, 3];
}
