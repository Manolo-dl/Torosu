import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { IconComponent, PageHeaderComponent, SectionTitleComponent } from '@shared/ui';
import { CollectionExportControlsComponent, CollectionListComponent, CollectionPreviewComponent, ExportCollectionsStore } from "@features/collection-exporter";

/** osu!'s collections out to a text file, or copied, in three steps. */
@Component({
  selector: 'app-export-collections',
  imports: [CollectionListComponent, CollectionExportControlsComponent, CollectionPreviewComponent, IconComponent, PageHeaderComponent, SectionTitleComponent, TranslocoPipe],
  templateUrl: './export-collections-page.component.html',
  styleUrl: './export-collections-page.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ExportCollectionsPageComponent {
  readonly store = inject(ExportCollectionsStore);
}
