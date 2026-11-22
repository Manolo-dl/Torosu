import { DecimalPipe } from '@angular/common';
import { afterRenderEffect, ChangeDetectionStrategy, Component, computed, inject, viewChild } from '@angular/core';
import { CdkVirtualScrollViewport, ScrollingModule } from '@angular/cdk/scrolling';
import { TranslocoPipe } from '@jsverse/transloco';
import { LanguageService } from '@shared/i18n';
import { OptionGroupComponent, OptionGroupItem, BadgeComponent, CardComponent, SkeletonComponent } from '@shared/ui';
import { TuiInput, TuiScrollControls, TuiScrollRef } from '@taiga-ui/core';
import { ReplayAnalysisStore, ScoreEntry } from '../../stores/replay-analysis.store';
import { GradeComponent, LocalScoresStore } from '@entities/local-scores';
import { OsuDbStore } from '@entities/osu-db';

/** The local plays with a replay, to pick the one to analyse. */
@Component({
    selector: 'app-score-list',
    imports: [GradeComponent, SkeletonComponent, DecimalPipe, OptionGroupComponent, ScrollingModule, TranslocoPipe, TuiInput, TuiScrollControls, TuiScrollRef, BadgeComponent, CardComponent],
    templateUrl: './score-list.component.html',
    styleUrl: './score-list.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ScoreListComponent {
    readonly store = inject(ReplayAnalysisStore);
    private readonly viewport = viewChild(CdkVirtualScrollViewport);
    private readonly scores = inject(LocalScoresStore);
    private readonly osuDb = inject(OsuDbStore);
    /** The plays, and their maps from osu!.db, are being read. */
    readonly loading = computed(() => this.scores.isLoading() || this.osuDb.isLoading());
    private readonly language = inject(LanguageService).current;

    constructor() {
        // The list is as tall as its plays: told to the virtual scroll, which
        // otherwise keeps rendering for the height it measured first.
        afterRenderEffect(() => {
            this.store.filtered().length;
            this.viewport()?.checkViewportSize();
        });
    }

    /** Dates in the language in use (Angular has no locale data but English's). */
    private readonly dates = computed(() => new Intl.DateTimeFormat(this.language(), { dateStyle: 'medium', timeStyle: 'short' }));

    /** "All", then each key count there are plays of. */
    readonly keyOptions = computed((): OptionGroupItem[] => [
        { label: 'replays.allKeys', translate: true },
        ...this.store.keyCounts().map(keys => ({ label: `${keys}K` })),
    ]);

    readonly keyIndex = computed(() => {
        const keys = this.store.keys();
        return keys === null ? 0 : this.store.keyCounts().indexOf(keys) + 1;
    });

    pickKeys(index: number) {
        this.store.setKeys(index === 0 ? null : this.store.keyCounts()[index - 1]);
    }

    search(event: Event) {
        this.store.setSearch((event.target as HTMLInputElement).value);
    }

    date(timestamp: number): string {
        return this.dates().format(timestamp);
    }

    trackEntry(_: number, entry: ScoreEntry): string {
        return entry.score.replay ?? '';
    }
}
