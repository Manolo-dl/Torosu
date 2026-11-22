import { ChangeDetectionStrategy, Component, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ButtonComponent, IconComponent } from '@shared/ui';
import { GAME_MODES } from '@shared/utils';

/** Before osu!.db is read: the modes bobbing, what reading it shows, and a button to. */
@Component({
    selector: 'app-home-library-invite',
    imports: [ButtonComponent, IconComponent, TranslocoPipe],
    template: `
        <div class="floating" aria-hidden="true">
            @for (mode of modes; track mode.name; let i = $index) {
                <span class="float" [style.--i]="i" [style.color]="mode.color"><app-icon [name]="mode.icon" /></span>
            }
        </div>
        <p>{{ 'home.library.hint' | transloco }}</p>
        <button appButton="soft" icon="library-big" type="button" (click)="read.emit()">
            {{ 'home.library.read' | transloco }}
        </button>
    `,
    styleUrl: './home-library-invite.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HomeLibraryInviteComponent {
    readonly read = output<void>();
    readonly modes = GAME_MODES;
}
