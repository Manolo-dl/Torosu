import { ChangeDetectionStrategy, Component, input, ViewEncapsulation } from '@angular/core';

export type NoticeTone = 'info' | 'success' | 'warning' | 'danger' | 'neutral' | 'accent';

/**
 * A message on the page, tinted by its tone, with the tone's icon:
 * `<div appNotice="warning">…</div>`. Its text can be plain or [innerHTML];
 * inside, `.notice-title` is set bold and `.notice-actions` holds buttons.
 */
@Component({
    selector: '[appNotice]',
    template: '<ng-content />',
    styleUrl: './notice.component.less',
    // Its styles reach the content it is given (the title, the actions), and
    // stay on .app-notice: a class, since [appNotice]="…" leaves no attribute.
    encapsulation: ViewEncapsulation.None,
    host: {
        class: 'app-notice',
        role: 'status',
        '[attr.data-tone]': 'appNotice() || "info"',
        '[attr.data-size]': 'size()',
    },
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NoticeComponent {
    readonly appNotice = input<NoticeTone | ''>('info');
    readonly size = input<'s' | 'm'>('m');
}
