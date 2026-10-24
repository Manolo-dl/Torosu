import { ChangeDetectionStrategy, Component, Input } from '@angular/core';
import { ScrollingModule } from '@angular/cdk/scrolling';
import { OsuBeatmapSet } from '@entities/osu-db/osu-db.model';
import { TuiScrollControls, TuiScrollRef } from '@taiga-ui/core';
import { BeatmapSetCardComponent } from '../beatmap-set-card/beatmap-set-card.component';

@Component({
  selector: 'app-beatmap-set-list',
  imports: [ScrollingModule, TuiScrollRef, TuiScrollControls, BeatmapSetCardComponent],
  templateUrl: './beatmap-set-list.component.html',
  styleUrl: './beatmap-set-list.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BeatmapSetListComponent {
  @Input() sets: OsuBeatmapSet[] = [];
  @Input() emptyText = 'No beatmaps';
  @Input() listHeight = 'calc(100vh - 320px)';

  trackSet(_: number, set: OsuBeatmapSet): string {
    return set.folderName;
  }
}
