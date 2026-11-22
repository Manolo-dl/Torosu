import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, DestroyRef, inject } from '@angular/core';
import { ImportFileDropzoneComponent, ImportPreviewComponent, ImportConfirmButtonComponent, ImportCollectionStore } from "@features/collection-importer";
import { IconComponent, PageHeaderComponent, SectionTitleComponent } from '@shared/ui';

/** Collections from a text file into osu!'s collection.db, in three steps. */
@Component({
  selector: 'app-import-collections-page',
  imports: [ImportFileDropzoneComponent, ImportPreviewComponent, ImportConfirmButtonComponent, IconComponent, PageHeaderComponent, SectionTitleComponent, TranslocoPipe],
  templateUrl: './import-collections-page.component.html',
  styleUrl: './import-collections-page.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ImportCollectionsPageComponent {
  readonly store = inject(ImportCollectionStore);

  constructor() {
    inject(DestroyRef).onDestroy(() => this.store.forgetResult());
  }
}
