import { CardComponent, TooltipDirective } from '@shared/ui';
import { DecimalPipe } from '@angular/common';
import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { JUDGEMENTS, ReplayAnalysis, maniaAccuracy } from '@entities/local-scores';
import { AnalysisHeroComponent } from '../analysis-hero/analysis-hero.component';
import { ScoreEntry, TimingView } from '../../stores/replay-analysis.store';

/** The play at a glance: its timing and its judgements. */
@Component({
    selector: 'app-analysis-summary',
    imports: [AnalysisHeroComponent, CardComponent, DecimalPipe, TooltipDirective, TranslocoPipe],
    templateUrl: './analysis-summary.component.html',
    styleUrl: './analysis-summary.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AnalysisSummaryComponent {
    readonly entry = input.required<ScoreEntry>();
    readonly analysis = input.required<ReplayAnalysis>();
    /** What the timing counts. */
    readonly view = input.required<TimingView>();

    readonly judgements = JUDGEMENTS;

    readonly accuracy = computed(() => {
        const a = this.analysis();
        return { game: maniaAccuracy(a.gameCounts, a.mods), ours: maniaAccuracy(a.stats.counts, a.mods) };
    });

    /** Of the timed hits, how many were early, 0 to 1. */
    readonly earlyShare = computed(() => {
        const timing = this.analysis().stats.timing[this.view()];
        const total = timing.early + timing.late;
        return total ? timing.early / total : 0.5;
    });

    /** Whether the analysis judged some notes differently than the game. */
    readonly differs = computed(() => {
        const a = this.analysis();
        return JUDGEMENTS.some(j => a.gameCounts[j] !== a.stats.counts[j]);
    });
}
