import { ChangeDetectionStrategy, Component } from '@angular/core';
import { CollectionListComponent, CollectionPreviewComponent, CollectionSummaryComponent, CollectionExportControlsComponent } from "@features/collection-exporter";

@Component({
  selector: 'app-export-collections',
  imports: [CollectionListComponent, CollectionSummaryComponent, CollectionPreviewComponent, CollectionExportControlsComponent],
  templateUrl: './export-collections-page.component.html',
  styleUrl: './export-collections-page.component.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ExportCollectionsPageComponent {}
