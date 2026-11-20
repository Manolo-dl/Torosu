import { ChangeDetectionStrategy, Component, booleanAttribute, input } from '@angular/core';
import { IconComponent } from '../icon/icon.component';

export type ButtonVariant = 'primary' | 'secondary' | 'soft' | 'ghost' | 'danger';
export type ButtonSize = 's' | 'm' | 'l';

/**
 * A button: `<button appButton="secondary" icon="download">…</button>`, also
 * on links. Without text and with an icon (`iconOnly`) it is square; the text
 * then names it for screen readers.
 */
@Component({
    selector: 'button[appButton], a[appButton]',
    imports: [IconComponent],
    template: `
        @if (loading()) {
            <span class="spinner" aria-hidden="true"></span>
        } @else if (icon()) {
            <app-icon [name]="icon()!" />
        }
        <span class="label" [class.hidden]="iconOnly()"><ng-content /></span>
        @if (iconEnd() && !loading()) {
            <app-icon [name]="iconEnd()!" />
        }
    `,
    styleUrl: './button.component.less',
    host: {
        '[attr.data-variant]': 'appButton() || "primary"',
        '[attr.data-size]': 'size()',
        '[class.icon-only]': 'iconOnly()',
        '[class.loading]': 'loading()',
        '[attr.aria-busy]': 'loading() || null',
    },
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ButtonComponent {
    readonly appButton = input<ButtonVariant | ''>('primary');
    readonly size = input<ButtonSize>('m');
    readonly icon = input<string>();
    readonly iconEnd = input<string>();
    readonly iconOnly = input(false, { transform: booleanAttribute });
    readonly loading = input(false, { transform: booleanAttribute });
}
