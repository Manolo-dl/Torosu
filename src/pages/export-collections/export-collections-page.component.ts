import { ChangeDetectionStrategy, Component } from '@angular/core';
import { PageHeaderComponent } from '@shared/ui';
import { CollectionListComponent, CollectionPreviewComponent, CollectionSummaryComponent, CollectionExportControlsComponent } from "@features/collection-exporter";

@Component({
  selector: 'app-export-collections',
  imports: [CollectionListComponent, CollectionSummaryComponent, CollectionPreviewComponent, CollectionExportControlsComponent, PageHeaderComponent],
  templateUrl: './export-collections-page.component.html',
  styleUrl: './export-collections-page.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ExportCollectionsPageComponent {}
