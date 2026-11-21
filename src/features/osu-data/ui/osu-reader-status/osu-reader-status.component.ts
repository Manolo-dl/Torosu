import { BadgeComponent } from '@shared/ui';
import { Component, inject, ChangeDetectionStrategy } from '@angular/core';
import { OsuReaderStore } from '@entities/osu-reader';

@Component({
    selector: 'app-osu-reader-status',
    imports: [BadgeComponent],
    templateUrl: './osu-reader-status.component.html',
    styleUrl: './osu-reader-status.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class OsuReaderStatusComponent {
    store = inject(OsuReaderStore);
}
