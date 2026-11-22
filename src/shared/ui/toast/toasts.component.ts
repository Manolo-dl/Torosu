import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { NotifyService, ToastTone } from '@shared/services';
import { IconComponent } from '../icon/icon.component';

const ICONS: Record<ToastTone, string> = { success: 'circle-check', danger: 'circle-x', info: 'info' };

/**
 * The notifications of `NotifyService`, at the bottom right. Each leaves when
 * its bar has run out, which stops while the pointer is on it. Only their
 * transform and opacity move, which the compositor does alone.
 */
@Component({
    selector: 'app-toasts',
    imports: [IconComponent, TranslocoPipe],
    template: `
        @for (toast of notify.toasts(); track toast.id) {
            <div
                class="toast"
                role="status"
                [attr.data-tone]="toast.tone"
                animate.enter="toast-enter"
                animate.leave="toast-leave"
            >
                <app-icon class="icon" [name]="icons[toast.tone]" />
                <p class="message">{{ toast.message }}</p>
                <button type="button" class="close" [attr.aria-label]="'settings.close' | transloco" (click)="notify.dismiss(toast.id)">
                    <app-icon name="x" />
                </button>
                <span class="timer" (animationend)="notify.dismiss(toast.id)"></span>
            </div>
        }
    `,
    styleUrl: './toasts.component.less',
    host: { 'aria-live': 'polite' },
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ToastsComponent {
    readonly notify = inject(NotifyService);
    readonly icons = ICONS;
}
