import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { BeatmapExporterStore } from '../../stores/beatmap-exporter.store';
import { TuiTitle } from '@taiga-ui/core';
import { TuiCardLarge } from '@taiga-ui/layout';
import { UncheckedNoticeComponent } from '@entities/osu-db/ui';

@Component({
  selector: 'app-beatmap-summary',
  imports: [TuiCardLarge, TuiTitle, UncheckedNoticeComponent],
  templateUrl: './beatmap-summary.component.html',
  styleUrl: './beatmap-summary.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BeatmapSummaryComponent {
  readonly store = inject(BeatmapExporterStore);
}
