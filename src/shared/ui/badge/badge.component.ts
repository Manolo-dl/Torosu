import { ChangeDetectionStrategy, Component, booleanAttribute, input } from '@angular/core';

export type BadgeTone = 'neutral' | 'accent' | 'accent-2' | 'success' | 'warning' | 'danger' | 'info';

/**
 * A small label: `<app-badge tone="accent">v1.3.0</app-badge>`, or on any
 * element, `<span appBadge tone="success">`. `dot` puts a dot of its colour
 * before the text, that pulses with `live`.
 */
@Component({
    selector: 'app-badge, [appBadge]',
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
        :host([data-size='s']) { block-size: 1.25rem; padding-inline: 0.45rem; font-size: 0.6875rem; }
        :host([data-size='l']) { block-size: 1.75rem; padding-inline: 0.7rem; font-size: var(--text-xs); }
        :host(.dot)::before {
            content: '';
            inline-size: 0.45em;
            block-size: 0.45em;
            margin-inline-end: 0.1em;
            border-radius: 50%;
            background: currentColor;
        }
        :host(.live)::before {
            animation: live 1.6s var(--ease-out) infinite;
        }
        @keyframes live {
            0% { box-shadow: 0 0 0 0 currentColor; }
            70%, 100% { box-shadow: 0 0 0 0.4em transparent; }
        }
        :host([data-tone='accent']) { --badge-bg: var(--accent-soft); --badge-fg: var(--accent-text); }
        :host([data-tone='accent-2']) { --badge-bg: var(--accent-2-soft); --badge-fg: var(--accent-2-text); }
        :host([data-tone='success']) { --badge-bg: var(--success-soft); --badge-fg: var(--success); }
        :host([data-tone='warning']) { --badge-bg: var(--warning-soft); --badge-fg: var(--warning); }
        :host([data-tone='danger']) { --badge-bg: var(--danger-soft); --badge-fg: var(--danger); }
        :host([data-tone='info']) { --badge-bg: var(--info-soft); --badge-fg: var(--info); }
    `,
    host: { '[attr.data-tone]': 'tone()', '[attr.data-size]': 'size()', '[class.dot]': 'dot() || live()', '[class.live]': 'live()' },
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BadgeComponent {
    readonly tone = input<BadgeTone>('neutral');
    readonly size = input<'s' | 'm' | 'l'>('m');
    readonly dot = input(false, { transform: booleanAttribute });
    readonly live = input(false, { transform: booleanAttribute });
}
