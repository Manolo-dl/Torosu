import { ChangeDetectionStrategy, Component, input } from '@angular/core';

/**
 * How far something has got: a native <progress> filled with both accents,
 * a light running along what is done, `<progress appProgress max="100" [value]="n">`.
 */
@Component({
    selector: 'progress[appProgress]',
    template: '',
    styles: `
        :host {
            --progress-height: 0.5rem;

            -webkit-appearance: none;
            appearance: none;
            display: block;
            inline-size: 100%;
            block-size: var(--progress-height);
            border: none;
            border-radius: var(--radius-pill);
            background: color-mix(in oklch, var(--text-1) 10%, transparent);
            overflow: hidden;
        }

        :host([data-size='s']) { --progress-height: 0.3rem; }

        :host::-webkit-progress-bar {
            background: transparent;
        }

        :host::-webkit-progress-value {
            border-radius: var(--radius-pill);
            background:
                linear-gradient(90deg, transparent 0%, oklch(1 0 0 / 0.35) 50%, transparent 100%) -100% 0 / 50% 100% no-repeat,
                linear-gradient(90deg, var(--accent), var(--accent-2));
            box-shadow: 0 0 10px var(--accent-glow);
            transition: inline-size var(--duration-3) var(--ease-out);
            animation: shine 1.6s var(--ease-in-out) infinite;
        }

        :host::-moz-progress-bar {
            border-radius: var(--radius-pill);
            background: linear-gradient(90deg, var(--accent), var(--accent-2));
        }

        @keyframes shine {
            to { background-position: 200% 0, 0 0; }
        }
    `,
    host: { '[attr.data-size]': 'size()' },
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ProgressComponent {
    readonly size = input<'s' | 'm'>('m');
}
