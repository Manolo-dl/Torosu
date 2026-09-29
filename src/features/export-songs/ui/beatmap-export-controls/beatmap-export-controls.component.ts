import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { BeatmapExporterStore } from '@features/export-songs/stores/beatmap-exporter.store';
import { BeatmapsetExportService } from '@shared/services';
import { ExportControlsComponent, ExportFormat } from '@shared/ui';
import { TuiTitle } from '@taiga-ui/core';
import { TuiCardLarge } from '@taiga-ui/layout';

const FORMATS: ExportFormat[] = [
  { value: 'urls', label: 'URLs' },
];

@Component({
  selector: 'app-beatmap-export-controls',
  imports: [ExportControlsComponent, TuiCardLarge, TuiTitle],
  templateUrl: './beatmap-export-controls.component.html',
  styleUrl: './beatmap-export-controls.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BeatmapExportControlsComponent {
  readonly store = inject(BeatmapExporterStore);
  readonly exportService = inject(BeatmapsetExportService);

  readonly formats = FORMATS;

  async export() {
    const lines = this.store.filteredBeatmaps()
      .map(s => this.exportService.formatLine(s.beatmapsetId, 'urls'));
    await this.exportService.saveToFile(lines.join('\n'), 'beatmaps.txt');
  }
}
