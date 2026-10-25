import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { CollectionStore } from '@entities/collection';
import { TuiTitle } from '@taiga-ui/core';
import { TuiCardLarge } from '@taiga-ui/layout';

@Component({
  selector: 'app-collection-summary',
  imports: [TuiCardLarge, TuiTitle],
  templateUrl: './collection-summary.component.html',
  styleUrl: './collection-summary.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CollectionSummaryComponent {
  readonly store = inject(CollectionStore);
}
