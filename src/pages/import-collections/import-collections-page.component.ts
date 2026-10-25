import { ChangeDetectionStrategy, Component } from '@angular/core';
import { ImportFileDropzoneComponent, ImportPreviewComponent, ImportConfirmButtonComponent } from "@features/collection-importer";
import { TuiTitle } from '@taiga-ui/core';

@Component({
  selector: 'app-import-collections-page',
  imports: [ImportFileDropzoneComponent, ImportPreviewComponent, ImportConfirmButtonComponent, TuiTitle],
  templateUrl: './import-collections-page.component.html',
  styleUrl: './import-collections-page.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ImportCollectionsPageComponent {}
