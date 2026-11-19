import { ChangeDetectionStrategy, Component, computed, inject, input } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ReplayAnalysis } from '@entities/local-scores';
import { OptionGroupComponent, OptionGroupItem } from '@shared/ui';
import { TuiTitle } from '@taiga-ui/core';
import { TuiCardLarge } from '@taiga-ui/layout';
import { ReplayAnalysisStore, ScoreEntry, TimingView } from '../../stores/replay-analysis.store';
import { AnalysisSummaryComponent } from '../analysis-summary/analysis-summary.component';
import { HitHistogramComponent } from '../hit-histogram/hit-histogram.component';
import { KeyStatsComponent } from '../key-stats/key-stats.component';
import { TimelineComponent } from '../timeline/timeline.component';

const VIEWS: TimingView[] = ['hits', 'releases'];

/** Everything about the analysed play: the summary, then the charts of the view picked. */
@Component({
    selector: 'app-analysis-panel',
    imports: [AnalysisSummaryComponent, HitHistogramComponent, KeyStatsComponent, OptionGroupComponent, TimelineComponent, TranslocoPipe, TuiCardLarge, TuiTitle],
    templateUrl: './analysis-panel.component.html',
    styleUrl: './analysis-panel.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AnalysisPanelComponent {
    readonly entry = input.required<ScoreEntry>();
    readonly analysis = input.required<ReplayAnalysis>();
    readonly store = inject(ReplayAnalysisStore);

    /** Releases only for maps with long notes that were let go in time. */
    readonly views = computed((): OptionGroupItem[] => [
        { label: 'replays.views.hits', translate: true },
        { label: 'replays.views.releases', translate: true, disabled: this.analysis().stats.releases.count === 0 },
    ]);

    readonly viewIndex = computed(() => VIEWS.indexOf(this.store.view()));

    pickView(index: number) {
        this.store.setView(VIEWS[index]);
    }
}
