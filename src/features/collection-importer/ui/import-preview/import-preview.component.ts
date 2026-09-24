import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { ImportCollectionStore } from '@features/collection-importer';
import { BeatmapSetCardComponent } from '@shared/ui';

@Component({
  selector: 'app-import-preview',
  imports: [BeatmapSetCardComponent],
  templateUrl: './import-preview.component.html',
  styleUrl: './import-preview.component.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ImportPreviewComponent {
  readonly store = inject(ImportCollectionStore);
}
