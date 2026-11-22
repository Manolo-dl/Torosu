import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { OsuReaderStatusComponent } from '@features/osu-data';
import { AnalysisPanelComponent, ReplayAnalysisStore, ScoreListComponent } from '@features/replay-analysis';
import { PageHeaderComponent, CardComponent, NoticeComponent, LoaderComponent } from '@shared/ui';

@Component({
    selector: 'app-replays-page',
    imports: [LoaderComponent, AnalysisPanelComponent, OsuReaderStatusComponent, PageHeaderComponent, ScoreListComponent, TranslocoPipe, CardComponent, NoticeComponent],
    templateUrl: './replays-page.component.html',
    styleUrl: './replays-page.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ReplaysPageComponent {
    readonly store = inject(ReplayAnalysisStore);
}
