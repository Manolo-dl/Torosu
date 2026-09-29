import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { CollectionStore } from '@entities/collection';
import { BeatmapSetListComponent } from '@shared/ui';
import { TuiTitle } from '@taiga-ui/core';
import { TuiBadge } from '@taiga-ui/kit';
import { TuiCardLarge } from '@taiga-ui/layout';

@Component({
  selector: 'app-collection-preview',
  imports: [BeatmapSetListComponent, TuiCardLarge, TuiTitle, TuiBadge],
  templateUrl: './collection-preview.component.html',
  styleUrl: './collection-preview.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CollectionPreviewComponent {
  readonly store = inject(CollectionStore);
}
