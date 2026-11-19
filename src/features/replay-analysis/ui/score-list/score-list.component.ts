import { DecimalPipe } from '@angular/common';
import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { ScrollingModule } from '@angular/cdk/scrolling';
import { TranslocoPipe } from '@jsverse/transloco';
import { LanguageService } from '@shared/i18n';
import { OptionGroupComponent, OptionGroupItem } from '@shared/ui';
import { TuiInput, TuiScrollControls, TuiScrollRef } from '@taiga-ui/core';
import { TuiBadge, TuiSwitch } from '@taiga-ui/kit';
import { TuiCardLarge } from '@taiga-ui/layout';
import { ReplayAnalysisStore, ScoreEntry } from '../../stores/replay-analysis.store';

/** The local plays with a replay, to pick the one to analyse. */
@Component({
    selector: 'app-score-list',
    imports: [DecimalPipe, OptionGroupComponent, ScrollingModule, TranslocoPipe, TuiBadge, TuiCardLarge, TuiInput, TuiScrollControls, TuiScrollRef, TuiSwitch],
    templateUrl: './score-list.component.html',
    styleUrl: './score-list.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ScoreListComponent {
    readonly store = inject(ReplayAnalysisStore);
    private readonly language = inject(LanguageService).current;

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

    follow(event: Event) {
        this.store.setFollow((event.target as HTMLInputElement).checked);
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
