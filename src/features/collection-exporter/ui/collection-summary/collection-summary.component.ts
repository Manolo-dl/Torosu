import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { CollectionStore } from '@entities/collection';
import { TuiTitle } from '@taiga-ui/core';
import { TuiCardLarge } from '@taiga-ui/layout';
import { UncheckedNoticeComponent } from '@entities/osu-db/ui';

@Component({
  selector: 'app-collection-summary',
  imports: [TuiCardLarge, TuiTitle, UncheckedNoticeComponent],
  templateUrl: './collection-summary.component.html',
  styleUrl: './collection-summary.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CollectionSummaryComponent {
  readonly store = inject(CollectionStore);
}
