import { ChangeDetectionStrategy, Component, input } from '@angular/core';

export type BadgeTone = 'neutral' | 'accent' | 'accent-2' | 'success' | 'warning' | 'danger' | 'info';

/** A small label: `<app-badge tone="accent">v1.3.0</app-badge>`. */
@Component({
    selector: 'app-badge',
    template: '<ng-content />',
    styles: `
        :host {
            --badge-bg: color-mix(in oklch, var(--text-1) 9%, transparent);
            --badge-fg: var(--text-2);

            display: inline-flex;
            align-items: center;
            gap: 0.3em;
            block-size: 1.5rem;
            padding-inline: 0.55rem;
            border-radius: var(--radius-pill);
            background: var(--badge-bg);
            color: var(--badge-fg);
            font-size: var(--text-2xs);
            font-weight: var(--weight-bold);
            letter-spacing: 0.02em;
            white-space: nowrap;
        }
        :host([data-size='s']) { block-size: 1.25rem; padding-inline: 0.45rem; }
        :host([data-tone='accent']) { --badge-bg: var(--accent-soft); --badge-fg: var(--accent-text); }
        :host([data-tone='accent-2']) { --badge-bg: var(--accent-2-soft); --badge-fg: var(--accent-2-text); }
        :host([data-tone='success']) { --badge-bg: var(--success-soft); --badge-fg: var(--success); }
        :host([data-tone='warning']) { --badge-bg: var(--warning-soft); --badge-fg: var(--warning); }
        :host([data-tone='danger']) { --badge-bg: var(--danger-soft); --badge-fg: var(--danger); }
        :host([data-tone='info']) { --badge-bg: var(--info-soft); --badge-fg: var(--info); }
    `,
    host: { '[attr.data-tone]': 'tone()', '[attr.data-size]': 'size()' },
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BadgeComponent {
    readonly tone = input<BadgeTone>('neutral');
    readonly size = input<'s' | 'm'>('m');
}
