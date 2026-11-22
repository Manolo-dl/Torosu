import { ButtonComponent } from '@shared/ui';
import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { ExportCollectionsStore } from '../../export-collections.store';

/** Lines of the text shown before "and N more". */
const PREVIEW_LINES = 14;

/** Step 3: the text as it will be written, saved to a .txt file or copied. */
@Component({
  selector: 'app-collection-preview',
  imports: [ButtonComponent, TranslocoPipe],
  templateUrl: './collection-preview.component.html',
  styleUrl: './collection-preview.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CollectionPreviewComponent {
  readonly store = inject(ExportCollectionsStore);

  readonly preview = computed(() => this.store.text().split('\n').slice(0, PREVIEW_LINES).join('\n'));
  readonly more = computed(() => Math.max(0, this.store.lines() - PREVIEW_LINES));
}
