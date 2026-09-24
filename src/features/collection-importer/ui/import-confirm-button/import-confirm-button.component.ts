import { Component, inject } from '@angular/core';
import { ImportCollectionStore } from '@features/collection-importer/import-collections.store';

@Component({
  selector: 'app-import-confirm-button',
  imports: [],
  templateUrl: './import-confirm-button.component.html',
  styleUrl: './import-confirm-button.component.css',
})
export class ImportConfirmButtonComponent {
  readonly store = inject(ImportCollectionStore);

  async onImport() {
    await this.store.importCollections();
  }
}
