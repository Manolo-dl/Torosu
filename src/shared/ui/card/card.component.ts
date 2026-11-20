import { ChangeDetectionStrategy, Component, booleanAttribute, input } from '@angular/core';

export type CardVariant = 'surface' | 'raised' | 'glass';

/**
 * A card: `<section appCard>…</section>`, also on links (`interactive` lifts
 * it on hover). `padding` is `none`, `s`, `m` or `l`.
 */
@Component({
    selector: '[appCard]',
    template: '<ng-content />',
    styleUrl: './card.component.less',
    host: {
        '[attr.data-variant]': 'appCard() || "surface"',
        '[attr.data-padding]': 'padding()',
        '[class.interactive]': 'interactive()',
    },
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CardComponent {
    readonly appCard = input<CardVariant | ''>('surface');
    readonly padding = input<'none' | 's' | 'm' | 'l'>('m');
    readonly interactive = input(false, { transform: booleanAttribute });
}
