import { A11yModule } from '@angular/cdk/a11y';
import { booleanAttribute, ChangeDetectionStrategy, Component, input, model, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { IconComponent } from '../icon/icon.component';

/**
 * A panel sliding in from the right over the app, `<app-drawer [(open)]="open"
 * heading="…">…</app-drawer>`. Its close button, Escape and a click outside
 * ask to close it (`closeRequest`), and it closes unless `autoClose` is
 * false. Only its transform and the backdrop's opacity move.
 */
@Component({
    selector: 'app-drawer',
    imports: [A11yModule, IconComponent, TranslocoPipe],
    template: `
        @if (open()) {
            <div class="backdrop" animate.enter="backdrop-enter" animate.leave="backdrop-leave" (click)="requestClose()"></div>
            <aside
                class="panel"
                role="dialog"
                aria-modal="true"
                [attr.aria-label]="heading()"
                cdkTrapFocus
                cdkTrapFocusAutoCapture
                animate.enter="panel-enter"
                animate.leave="panel-leave"
                (keydown.escape)="requestClose()"
            >
                <header>
                    <div class="titles">
                        <!-- Focused as it opens: from there Tab goes through it. -->
                        <h2 tabindex="-1" cdkFocusInitial>{{ heading() }}</h2>
                        @if (subtitle(); as subtitle) {
                            <p>{{ subtitle }}</p>
                        }
                    </div>
                    <ng-content select="[appDrawerStatus]" />
                    <button type="button" class="close" [attr.aria-label]="'settings.close' | transloco" (click)="requestClose()">
                        <app-icon name="x" />
                    </button>
                </header>
                <ng-content select="[appDrawerNav]" />
                <div class="body">
                    <ng-content />
                </div>
            </aside>
        }
    `,
    styleUrl: './drawer.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DrawerComponent {
    readonly open = model.required<boolean>();
    readonly heading = input.required<string>();
    readonly subtitle = input<string>();
    /**
     * Whether it closes itself when asked; when false the parent closes it
     * on `closeRequest` (after saving, say).
     */
    readonly autoClose = input(true, { transform: booleanAttribute });
    /** Its close button, Escape or a click outside asked to close it. */
    readonly closeRequest = output<void>();

    requestClose() {
        this.closeRequest.emit();
        if (this.autoClose()) this.open.set(false);
    }
}
