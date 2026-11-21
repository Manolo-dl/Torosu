import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { CollectionStore } from '@entities/collection';
import { CollectionExportService } from '../../services/collection-export.service';
import { ExportControlsComponent, ExportFormat, CardComponent } from '@shared/ui';
import { TuiTitle } from '@taiga-ui/core';

const FORMATS: ExportFormat[] = [
  { value: 'urls',   label: 'collections.export.links' },
  { value: 'import', label: 'collections.export.importFormat' },
];

@Component({
  selector: 'app-collection-export-controls',
  imports: [ExportControlsComponent, TuiTitle, TranslocoPipe, CardComponent],
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
