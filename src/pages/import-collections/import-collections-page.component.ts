import { ChangeDetectionStrategy, Component } from '@angular/core';
import { ImportFileDropzoneComponent, ImportPreviewComponent, ImportConfirmButtonComponent } from "@features/collection-importer";
import { PageHeaderComponent } from '@shared/ui';

@Component({
  selector: 'app-import-collections-page',
  imports: [ImportFileDropzoneComponent, ImportPreviewComponent, ImportConfirmButtonComponent, PageHeaderComponent],
  templateUrl: './import-collections-page.component.html',
  styleUrl: './import-collections-page.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ImportCollectionsPageComponent {}
