import { ChangeDetectionStrategy, Component } from '@angular/core';

/**
 * The heading of a part of a page: small capitals over a hairline that runs
 * to the edge, `<h2 appSectionTitle>Queue</h2>`. What follows the text (a
 * count, a button) sits after it, before the line.
 */
@Component({
    selector: '[appSectionTitle]',
    template: '<ng-content />',
    styles: `
        :host {
            display: flex;
            align-items: center;
            gap: var(--space-2);
            margin: 0 0 var(--space-3);
            color: var(--text-3);
            font-family: var(--font-display);
            font-size: var(--text-2xs);
            font-weight: var(--weight-bold);
            letter-spacing: 0.09em;
            line-height: 1.5rem;
            text-transform: uppercase;
        }

        :host::after {
            content: '';
            flex: 1;
            min-inline-size: var(--space-4);
            block-size: 1px;
            background: linear-gradient(90deg, var(--line), transparent);
        }
    `,
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SectionTitleComponent {}
