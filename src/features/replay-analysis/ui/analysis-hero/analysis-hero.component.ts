import { DecimalPipe } from '@angular/common';
import { ChangeDetectionStrategy, Component, input } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { GradeComponent, ReplayAnalysis } from '@entities/local-scores';
import { BadgeComponent } from '@shared/ui';
import { ScoreEntry } from '../../stores/replay-analysis.store';

/** The play analysed: its grade, its map, its mods and player, its accuracy. */
@Component({
    selector: 'app-analysis-hero',
    imports: [BadgeComponent, DecimalPipe, GradeComponent, TranslocoPipe],
    templateUrl: './analysis-hero.component.html',
    styleUrl: './analysis-hero.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AnalysisHeroComponent {
    readonly entry = input.required<ScoreEntry>();
    readonly analysis = input.required<ReplayAnalysis>();
    /** As osu! counts it. */
    readonly accuracy = input.required<number>();
}
