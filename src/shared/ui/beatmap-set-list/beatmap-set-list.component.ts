import { ChangeDetectionStrategy, Component, input, TemplateRef } from '@angular/core';
import { ScrollingModule } from '@angular/cdk/scrolling';
import { OsuBeatmapSet } from '@entities/osu-db/osu-db.model';
import { TuiScrollControls, TuiScrollRef } from '@taiga-ui/core';
import { BeatmapSetCardComponent, DiffActionContext } from '../beatmap-set-card/beatmap-set-card.component';

@Component({
  selector: 'app-beatmap-set-list',
  imports: [ScrollingModule, TuiScrollRef, TuiScrollControls, BeatmapSetCardComponent],
  templateUrl: './beatmap-set-list.component.html',
  styleUrl: './beatmap-set-list.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BeatmapSetListComponent {
  readonly sets = input<OsuBeatmapSet[]>([]);
  readonly emptyText = input('No beatmaps');
  readonly listHeight = input('calc(100vh - 320px)');
  /** For each card: see `BeatmapSetCardComponent.diffAction`. */
  readonly diffAction = input<TemplateRef<DiffActionContext>>();

  trackSet(_: number, set: OsuBeatmapSet): string {
    return set.folderName;
  }
}
