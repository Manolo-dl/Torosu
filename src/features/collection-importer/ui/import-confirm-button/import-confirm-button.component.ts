import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { ImportCollectionStore } from '@features/collection-importer/import-collections.store';
import { TuiButton } from '@taiga-ui/core';
import { TuiButtonLoading } from '@taiga-ui/kit';

@Component({
  selector: 'app-import-confirm-button',
  imports: [TuiButton, TuiButtonLoading],
  templateUrl: './import-confirm-button.component.html',
  styleUrl: './import-confirm-button.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ImportConfirmButtonComponent {
  readonly store = inject(ImportCollectionStore);

  async onImport() {
    await this.store.importCollections();
  }
}
