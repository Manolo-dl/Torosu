import { Component } from '@angular/core';
import { ImportFileDropzoneComponent, ImportPreviewComponent, ImportConfirmButtonComponent } from "@features/collection-importer";

@Component({
  selector: 'app-import-collections-page.component',
  imports: [ImportFileDropzoneComponent, ImportPreviewComponent, ImportConfirmButtonComponent],
  templateUrl: './import-collections-page.component.html',
  styleUrl: './import-collections-page.component.css',
})
export class ImportCollectionsPageComponent {}
