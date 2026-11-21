import { TranslocoPipe } from '@jsverse/transloco';
import { Component, ChangeDetectionStrategy, inject } from '@angular/core';
import { OsuReaderStore } from '@entities/osu-reader';
import { OsuReaderStatusComponent, OsuDataSectionsComponent } from '@features/osu-data';
import { PageHeaderComponent, NoticeComponent } from '@shared/ui';

@Component({
    selector: 'app-osu-data-page',
    imports: [TranslocoPipe, OsuReaderStatusComponent, OsuDataSectionsComponent, PageHeaderComponent, NoticeComponent],
    templateUrl: './osu-data-page.component.html',
    styleUrl: './osu-data-page.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class OsuDataPage {
    readonly store = inject(OsuReaderStore);
}
