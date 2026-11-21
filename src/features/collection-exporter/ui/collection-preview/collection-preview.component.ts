import { BadgeComponent, CardComponent } from '@shared/ui';
import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { CollectionStore } from '@entities/collection';
import { BeatmapSetListComponent } from '@entities/osu-db/ui';
import { TuiTitle } from '@taiga-ui/core';

@Component({
  selector: 'app-collection-preview',
  imports: [BeatmapSetListComponent, TuiTitle, TranslocoPipe, BadgeComponent, CardComponent],
  templateUrl: './collection-preview.component.html',
  styleUrl: './collection-preview.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CollectionPreviewComponent {
  readonly store = inject(CollectionStore);
}
