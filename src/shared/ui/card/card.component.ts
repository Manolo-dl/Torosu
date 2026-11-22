import { ChangeDetectionStrategy, Component, booleanAttribute, input } from '@angular/core';

export type CardVariant = 'surface' | 'raised' | 'glass';

/**
 * A card: `<section appCard>…</section>`, also on links (`interactive` lifts
 * it on hover, with a glow under the pointer). `padding` is `none`, `s`, `m`
 * or `l`.
 */
@Component({
    selector: '[appCard]',
    template: '<ng-content />',
    styleUrl: './card.component.less',
    host: {
        '[attr.data-variant]': 'appCard() || "surface"',
        '[attr.data-padding]': 'padding()',
        '[class.interactive]': 'interactive()',
        '(pointermove)': 'follow($event)',
    },
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CardComponent {
    readonly appCard = input<CardVariant | ''>('surface');
    readonly padding = input<'none' | 's' | 'm' | 'l'>('m');
    readonly interactive = input(false, { transform: booleanAttribute });

    /** Where the pointer is, for the glow of an interactive card. */
    follow(event: PointerEvent) {
        if (!this.interactive()) return;
        const card = event.currentTarget as HTMLElement;
        const box = card.getBoundingClientRect();
        card.style.setProperty('--pointer-x', `${event.clientX - box.left}px`);
        card.style.setProperty('--pointer-y', `${event.clientY - box.top}px`);
    }
}
