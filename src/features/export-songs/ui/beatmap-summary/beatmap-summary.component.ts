import { CardComponent, SkeletonComponent } from '@shared/ui';
import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { BeatmapExporterStore } from '../../stores/beatmap-exporter.store';
import { TuiTitle } from '@taiga-ui/core';
import { UncheckedNoticeComponent } from '@entities/osu-db/ui';
import { OsuDbStore } from '@entities/osu-db';

@Component({
  selector: 'app-beatmap-summary',
  imports: [SkeletonComponent, TuiTitle, UncheckedNoticeComponent, TranslocoPipe, CardComponent],
  templateUrl: './beatmap-summary.component.html',
  styleUrl: './beatmap-summary.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BeatmapSummaryComponent {
  readonly store = inject(BeatmapExporterStore);
  /** The count comes from osu!.db. */
  readonly osuDb = inject(OsuDbStore);
}
