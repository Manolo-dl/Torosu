import { ButtonComponent, IconComponent, NoticeComponent } from '@shared/ui';
import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, DestroyRef, inject } from '@angular/core';
import { ImportCollectionStore } from '../../import-collections.store';

/** How often whether osu! is running is checked, while the page is open. */
const OSU_CHECK_MS = 2000;

/**
 * Step 3: the import, once osu! is closed (it keeps the collections in
 * memory and writes collection.db over when it closes, so what is imported
 * meanwhile would be lost), and what it changed.
 */
@Component({
  selector: 'app-import-confirm-button',
  imports: [ButtonComponent, IconComponent, NoticeComponent, TranslocoPipe],
  templateUrl: './import-confirm-button.component.html',
  styleUrl: './import-confirm-button.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ImportConfirmButtonComponent {
  readonly store = inject(ImportCollectionStore);

  constructor() {
    this.store.checkOsu();
    const timer = setInterval(() => this.store.checkOsu(), OSU_CHECK_MS);
    inject(DestroyRef).onDestroy(() => clearInterval(timer));
  }
}
