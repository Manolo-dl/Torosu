import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';

/**
 * An icon of the set the app ships (Lucide, in assets/taiga-ui/icons), drawn
 * in the text's colour and size: `<app-icon name="house" />`. Taiga's names
 * (`@tui.house`) work too.
 */
@Component({
    selector: 'app-icon',
    template: '',
    styles: `
        /* The icons' strokes are as wide as their image is wider than tall
           (stroke-width: calc((100vw - 100vh) / 10)), as Taiga draws them:
           the mask is stretched by ten times --icon-stroke. A square mask
           (contain) leaves them invisible. */
        :host {
            --icon-stroke: 0.125rem;

            display: inline-block;
            flex-shrink: 0;
            inline-size: 1em;
            block-size: 1em;
            background-color: currentColor;
            vertical-align: middle;
            -webkit-mask-image: var(--icon);
            mask-image: var(--icon);
            -webkit-mask-size: calc(100% + 10 * var(--icon-stroke)) 100%;
            mask-size: calc(100% + 10 * var(--icon-stroke)) 100%;
            -webkit-mask-position: center;
            mask-position: center;
            -webkit-mask-repeat: no-repeat;
            mask-repeat: no-repeat;
        }
    `,
    host: { 'aria-hidden': 'true', '[style.--icon]': 'url()' },
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class IconComponent {
    readonly name = input.required<string>();

    readonly url = computed(() => `url("assets/taiga-ui/icons/${this.name().replace(/^@tui\./, '')}.svg")`);
}
