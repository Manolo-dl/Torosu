import { ButtonComponent } from '@shared/ui';
import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { ImportCollectionStore } from '../../import-collections.store';

@Component({
  selector: 'app-import-confirm-button',
  imports: [TranslocoPipe, ButtonComponent],
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
