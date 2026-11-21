import { ChangeDetectionStrategy, Component, input } from '@angular/core';
import { Grade } from '../../local-scores.model';

/** A play's grade, in osu!'s colours for it: `<app-grade [grade]="grade" size="l" />`. */
@Component({
    selector: 'app-grade',
    template: '{{ grade().letter }}',
    styles: `
        :host {
            --grade: #ff5a5a;

            display: inline-grid;
            place-items: center;
            flex-shrink: 0;
            inline-size: 2.25rem;
            block-size: 2.25rem;
            border-radius: var(--radius-md);
            background: color-mix(in srgb, var(--grade) 14%, transparent);
            box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--grade) 35%, transparent);
            color: var(--grade);
            font-family: var(--font-display);
            font-size: 1.05rem;
            font-weight: var(--weight-black);
            letter-spacing: -0.04em;
            text-shadow: 0 0 12px color-mix(in srgb, var(--grade) 45%, transparent);
        }

        :host([data-size='l']) {
            inline-size: 4.25rem;
            block-size: 4.25rem;
            border-radius: var(--radius-lg);
            font-size: 2.1rem;
            animation: grade-in var(--duration-4) var(--ease-spring) backwards;
        }

        :host([data-letter='SS']), :host([data-letter='S']) { --grade: #ffcc33; }
        :host([data-letter='A']) { --grade: #7fd43a; }
        :host([data-letter='B']) { --grade: #4aa8ff; }
        :host([data-letter='C']) { --grade: #c47bff; }
        :host(.silver) { --grade: #d8e0ea; }

        @keyframes grade-in {
            from { opacity: 0; transform: scale(1.6) rotate(-8deg); }
        }
    `,
    host: {
        '[attr.data-letter]': 'grade().letter',
        '[attr.data-size]': 'size()',
        '[class.silver]': 'grade().silver',
        '[attr.aria-label]': 'grade().letter',
    },
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class GradeComponent {
    readonly grade = input.required<Grade>();
    readonly size = input<'m' | 'l'>('m');
}
