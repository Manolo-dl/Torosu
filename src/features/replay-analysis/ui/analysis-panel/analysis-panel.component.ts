import { ChangeDetectionStrategy, Component, computed, inject, input } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ReplayAnalysis } from '@entities/local-scores';
import { OptionGroupComponent, OptionGroupItem, CardComponent } from '@shared/ui';
import { TuiTitle } from '@taiga-ui/core';
import { ReplayAnalysisStore, ScoreEntry, TimingView } from '../../stores/replay-analysis.store';
import { AnalysisSummaryComponent } from '../analysis-summary/analysis-summary.component';
import { HitHistogramComponent } from '../hit-histogram/hit-histogram.component';
import { KeyStatsComponent } from '../key-stats/key-stats.component';
import { TimelineComponent } from '../timeline/timeline.component';

const VIEWS: TimingView[] = ['all', 'rice', 'ln'];

/** Everything about the analysed play: the summary, then the charts of the view picked. */
@Component({
    selector: 'app-analysis-panel',
    imports: [AnalysisSummaryComponent, HitHistogramComponent, KeyStatsComponent, OptionGroupComponent, TimelineComponent, TranslocoPipe, TuiTitle, CardComponent],
    templateUrl: './analysis-panel.component.html',
    styleUrl: './analysis-panel.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AnalysisPanelComponent {
    readonly entry = input.required<ScoreEntry>();
    readonly analysis = input.required<ReplayAnalysis>();
    readonly store = inject(ReplayAnalysisStore);

    /** Rice and LN only for maps that have them. */
    readonly views = computed((): OptionGroupItem[] => {
        const timing = this.analysis().stats.timing;
        return [
            { label: 'replays.views.all', translate: true },
            { label: 'replays.views.rice', translate: true, disabled: timing.rice.count === 0 },
            { label: 'replays.views.ln', translate: true, disabled: timing.ln.count === 0 },
        ];
    });

    readonly viewIndex = computed(() => VIEWS.indexOf(this.store.view()));

    pickView(index: number) {
        this.store.setView(VIEWS[index]);
    }
}
