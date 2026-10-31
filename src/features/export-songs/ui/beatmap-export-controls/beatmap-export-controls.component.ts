import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { BeatmapExporterStore } from '../../stores/beatmap-exporter.store';
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
    // A set downloaded twice is in two folders with the same id.
    const ids = new Set(this.store.filteredBeatmaps().map(s => s.beatmapsetId));
    const lines = [...ids].map(id => this.exportService.formatLine(id, 'urls'));
    await this.exportService.saveToFile(lines.join('\n'), 'beatmaps.txt');
  }
}
