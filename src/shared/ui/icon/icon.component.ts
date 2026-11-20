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
        :host {
            display: inline-block;
            flex-shrink: 0;
            inline-size: 1em;
            block-size: 1em;
            background: currentColor;
            -webkit-mask: var(--icon) center / contain no-repeat;
            mask: var(--icon) center / contain no-repeat;
        }
    `,
    host: { 'aria-hidden': 'true', '[style.--icon]': 'url()' },
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class IconComponent {
    readonly name = input.required<string>();

    readonly url = computed(() => `url("assets/taiga-ui/icons/${this.name().replace(/^@tui\./, '')}.svg")`);
}
