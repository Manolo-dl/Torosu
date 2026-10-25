import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { BeatmapExporterStore } from '@features/export-songs/stores/beatmap-exporter.store';
import { TuiTitle } from '@taiga-ui/core';
import { TuiCardLarge } from '@taiga-ui/layout';

@Component({
  selector: 'app-beatmap-summary',
  imports: [TuiCardLarge, TuiTitle],
  templateUrl: './beatmap-summary.component.html',
  styleUrl: './beatmap-summary.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BeatmapSummaryComponent {
  readonly store = inject(BeatmapExporterStore);
}
