import { Component, ChangeDetectionStrategy, inject } from '@angular/core';
import { OsuReaderStore } from '@entities/osu-reader';
import { OsuReaderStatusComponent, OsuDataSectionsComponent } from '@features/osu-data';
import { TuiNotificationDirective, TuiTitle } from '@taiga-ui/core';

@Component({
    selector: 'app-osu-data-page',
    imports: [OsuReaderStatusComponent, OsuDataSectionsComponent, TuiNotificationDirective, TuiTitle],
    templateUrl: './osu-data-page.component.html',
    styleUrl: './osu-data-page.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class OsuDataPage {
    readonly store = inject(OsuReaderStore);
}
