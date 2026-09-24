import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { CollectionStore } from '@entities/collection';
import { BeatmapSetListComponent } from '@shared/ui';

@Component({
  selector: 'app-collection-preview',
  imports: [BeatmapSetListComponent],
  templateUrl: './collection-preview.component.html',
  styleUrl: './collection-preview.component.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CollectionPreviewComponent {
  readonly store = inject(CollectionStore);
}
