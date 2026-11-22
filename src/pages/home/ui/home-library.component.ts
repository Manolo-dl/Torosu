import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { isSubmitted, OsuDbStore } from '@entities/osu-db';
import { CountUpDirective } from '@shared/directives';
import { BadgeComponent, IconComponent, SectionTitleComponent, SkeletonComponent } from '@shared/ui';
import { GAME_MODES } from '@shared/utils';
import { HomeLibraryInviteComponent } from './home-library-invite.component';

interface ModeStats {
    /** A translation key. */
    name: string;
    icon: string;
    color: string;
    diffs: number;
    sets: number;
    /** Of all the diffs, 0 to 1. */
    share: number;
}

/**
 * What is in the Songs folder, from osu!.db: the diffs of each mode, how many
 * are ranked, loved, not uploaded and played. Read when asked, or already
 * read by a tool.
 */
@Component({
    selector: 'app-home-library',
    imports: [BadgeComponent, CountUpDirective, HomeLibraryInviteComponent, IconComponent, SectionTitleComponent, SkeletonComponent, TranslocoPipe],
    templateUrl: './home-library.component.html',
    styleUrl: './home-library.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HomeLibraryComponent {
    readonly osuDb = inject(OsuDbStore);
    readonly modes = GAME_MODES;

    readonly stats = computed(() => {
        const sets = this.osuDb.beatmapSets();
        const perMode = GAME_MODES.map(() => ({ diffs: 0, sets: 0 }));
        let diffs = 0;
        let ranked = 0;
        let loved = 0;
        let unsubmitted = 0;
        let played = 0;

        for (const set of sets) {
            const modes = new Set<number>();
            for (const diff of set.diffs) {
                diffs++;
                if (perMode[diff.mode]) perMode[diff.mode].diffs++;
                modes.add(diff.mode);
                if (diff.status === 'ranked' || diff.status === 'approved') ranked++;
                else if (diff.status === 'loved') loved++;
                if (!isSubmitted(diff)) unsubmitted++;
                if (diff.lastPlayed) played++;
            }
            for (const mode of modes) if (perMode[mode]) perMode[mode].sets++;
        }

        const modes: ModeStats[] = GAME_MODES.map((mode, i) => ({
            name: mode.key,
            icon: mode.icon,
            color: mode.color,
            ...perMode[i],
            share: diffs ? perMode[i].diffs / diffs : 0,
        }));
        return { sets: sets.length, diffs, ranked, loved, unsubmitted, played, playedShare: diffs ? played / diffs : 0, modes };
    });
}
