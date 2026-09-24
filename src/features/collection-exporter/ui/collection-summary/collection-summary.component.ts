import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { CollectionStore } from '@entities/collection';

@Component({
  selector: 'app-collection-summary',
  imports: [],
  templateUrl: './collection-summary.component.html',
  styleUrl: './collection-summary.component.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CollectionSummaryComponent {
  readonly store = inject(CollectionStore);
}
