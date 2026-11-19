import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { AnalysisPanelComponent, ReplayAnalysisStore, ScoreListComponent } from '@features/replay-analysis';
import { PageHeaderComponent } from '@shared/ui';
import { TuiLoader, TuiNotificationDirective } from '@taiga-ui/core';
import { TuiCardLarge } from '@taiga-ui/layout';

@Component({
    selector: 'app-replays-page',
    imports: [AnalysisPanelComponent, PageHeaderComponent, ScoreListComponent, TranslocoPipe, TuiCardLarge, TuiLoader, TuiNotificationDirective],
    templateUrl: './replays-page.component.html',
    styleUrl: './replays-page.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ReplaysPageComponent {
    readonly store = inject(ReplayAnalysisStore);
}
