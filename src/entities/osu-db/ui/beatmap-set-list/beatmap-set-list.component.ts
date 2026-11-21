import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, computed, inject, input, TemplateRef } from '@angular/core';
import { ScrollingModule } from '@angular/cdk/scrolling';
import { OsuBeatmapSet } from '../../osu-db.model';
import { TuiScrollControls, TuiScrollRef } from '@taiga-ui/core';
import { BeatmapSetCardComponent, DiffActionContext } from '../beatmap-set-card/beatmap-set-card.component';
import { ButtonComponent, SkeletonComponent } from '@shared/ui';
import { OsuDbStore } from '../../osu-db.store';

@Component({
  selector: 'app-beatmap-set-list',
  imports: [ScrollingModule, TuiScrollRef, TuiScrollControls, BeatmapSetCardComponent, ButtonComponent, SkeletonComponent, TranslocoPipe],
  templateUrl: './beatmap-set-list.component.html',
  styleUrl: './beatmap-set-list.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BeatmapSetListComponent {
  readonly sets = input<OsuBeatmapSet[]>([]);
  /** A translation key. */
  readonly emptyText = input('beatmaps.none');
  readonly listHeight = input('calc(100vh - 320px)');
  /** For each card: see `BeatmapSetCardComponent.diffAction`. */
  readonly diffAction = input<TemplateRef<DiffActionContext>>();

  readonly osuDb = inject(OsuDbStore);
  /** The sets come from osu!.db: while it is read, their shapes show. */
  readonly loading = computed(() => this.osuDb.isLoading() && !this.osuDb.isLoaded());
  /** It could not be read: it can be tried again. */
  readonly failed = computed(() => !this.osuDb.isLoading() && !this.osuDb.isLoaded());
  readonly skeletonRows = Array.from({ length: 8 }, (_, i) => i);

  trackSet(_: number, set: OsuBeatmapSet): string {
    return set.folderName;
  }
}
