import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { BeatmapExporterStore } from '@features/export-songs/stores/beatmap-exporter.store';

@Component({
  selector: 'app-beatmap-summary',
  imports: [],
  templateUrl: './beatmap-summary.component.html',
  styleUrl: './beatmap-summary.component.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BeatmapSummaryComponent {
  readonly store = inject(BeatmapExporterStore);
}
