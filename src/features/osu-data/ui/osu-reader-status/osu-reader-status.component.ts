import { Component, inject, ChangeDetectionStrategy } from '@angular/core';
import { OsuReaderStore } from '@entities/osu-reader';
import { TuiBadge, TuiStatus } from '@taiga-ui/kit';

@Component({
    selector: 'app-osu-reader-status',
    imports: [TuiBadge, TuiStatus],
    templateUrl: './osu-reader-status.component.html',
    styleUrl: './osu-reader-status.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class OsuReaderStatusComponent {
    store = inject(OsuReaderStore);
}
