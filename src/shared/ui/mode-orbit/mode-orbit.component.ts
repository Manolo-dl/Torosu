import { ChangeDetectionStrategy, Component } from '@angular/core';
import { GAME_MODES } from '@shared/utils';
import { IconComponent } from '../icon/icon.component';

/**
 * The four game modes' icons orbiting a hit circle, its approach circle
 * closing in on it again and again: an ornament. Still with reduced motion.
 */
@Component({
    selector: 'app-mode-orbit',
    imports: [IconComponent],
    template: `
        <span class="track"></span>
        <span class="hit">
            <span class="approach"></span>
        </span>
        <span class="orbit">
            @for (mode of modes; track mode.name; let i = $index) {
                <span class="mode" [style.--i]="i" [style.--mode-color]="mode.color" [title]="mode.name">
                    <app-icon [name]="mode.icon" />
                </span>
            }
        </span>
    `,
    styleUrl: './mode-orbit.component.less',
    host: { 'aria-hidden': 'true' },
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ModeOrbitComponent {
    readonly modes = GAME_MODES;
}
