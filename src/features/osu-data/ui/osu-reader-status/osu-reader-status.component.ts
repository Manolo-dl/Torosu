import { BadgeComponent } from '@shared/ui';
import { TranslocoPipe } from '@jsverse/transloco';
import { Component, inject, ChangeDetectionStrategy } from '@angular/core';
import { OsuReaderStore } from '@entities/osu-reader';

@Component({
    selector: 'app-osu-reader-status',
    imports: [BadgeComponent, TranslocoPipe],
    templateUrl: './osu-reader-status.component.html',
    styleUrl: './osu-reader-status.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class OsuReaderStatusComponent {
    store = inject(OsuReaderStore);
}
