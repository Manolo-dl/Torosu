import { ChangeDetectionStrategy, Component, input } from '@angular/core';

/**
 * On and off: a native checkbox drawn as a switch, `<input type="checkbox"
 * appSwitch [(ngModel)]="on" />`. Being the input itself, forms, `checked`
 * and labels work as with any checkbox.
 */
@Component({
    selector: 'input[type="checkbox"][appSwitch]',
    template: '',
    styleUrl: './switch.component.less',
    host: { '[attr.data-size]': 'size()' },
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SwitchComponent {
    readonly size = input<'s' | 'm'>('m');
}

/** A native checkbox in the app's style: `<input type="checkbox" appCheckbox />`. */
@Component({
    selector: 'input[type="checkbox"][appCheckbox]',
    template: '',
    styleUrl: './checkbox.component.less',
    host: { '[attr.data-size]': 'size()' },
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CheckboxComponent {
    readonly size = input<'s' | 'm'>('m');
}
