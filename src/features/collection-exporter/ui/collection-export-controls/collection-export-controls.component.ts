import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { CollectionStore } from '@entities/collection';
import { CollectionExportService } from '@features/collection-exporter/services/collection-export.service';
import { ExportControlsComponent, ExportFormat } from '@shared/ui';
import { TuiTitle } from '@taiga-ui/core';
import { TuiCardLarge } from '@taiga-ui/layout';

const FORMATS: ExportFormat[] = [
  { value: 'urls',   label: 'Download links' },
  { value: 'import', label: 'Import format' },
];

@Component({
  selector: 'app-collection-export-controls',
  imports: [ExportControlsComponent, TuiCardLarge, TuiTitle],
  templateUrl: './collection-export-controls.component.html',
  styleUrl: './collection-export-controls.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CollectionExportControlsComponent {
  readonly exportService = inject(CollectionExportService);
  readonly store = inject(CollectionStore);
  readonly formats = FORMATS;

  async export(format: string) {
    await this.exportService.exportToFile(format as 'urls' | 'import');
  }
}
