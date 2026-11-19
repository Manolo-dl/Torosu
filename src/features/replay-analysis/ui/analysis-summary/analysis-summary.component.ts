import { DecimalPipe } from '@angular/common';
import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { JUDGEMENTS, ReplayAnalysis, maniaAccuracy } from '@entities/local-scores';
import { TuiHint } from '@taiga-ui/core';
import { TuiCardLarge } from '@taiga-ui/layout';
import { ScoreEntry } from '../../stores/replay-analysis.store';

/** The play at a glance: its timing and its judgements. */
@Component({
    selector: 'app-analysis-summary',
    imports: [DecimalPipe, TranslocoPipe, TuiCardLarge, TuiHint],
    templateUrl: './analysis-summary.component.html',
    styleUrl: './analysis-summary.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AnalysisSummaryComponent {
    readonly entry = input.required<ScoreEntry>();
    readonly analysis = input.required<ReplayAnalysis>();
    /** Count the long note releases with the presses, as osu! does. */
    readonly includeReleases = input(true);

    readonly judgements = JUDGEMENTS;

    readonly accuracy = computed(() => {
        const a = this.analysis();
        return { game: maniaAccuracy(a.gameCounts, a.mods), ours: maniaAccuracy(a.stats.counts, a.mods) };
    });

    /** Whether the analysis judged some notes differently than the game. */
    readonly differs = computed(() => {
        const a = this.analysis();
        return JUDGEMENTS.some(j => a.gameCounts[j] !== a.stats.counts[j]);
    });
}
