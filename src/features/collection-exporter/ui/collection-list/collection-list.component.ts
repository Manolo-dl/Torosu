import { BadgeComponent, CardComponent, CheckboxComponent, SkeletonComponent } from '@shared/ui';
import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { CollectionStore } from '@entities/collection';
import { OsuDbStore } from '@entities/osu-db';
import { TuiCell, TuiLabel, TuiScrollbar, TuiTitle } from '@taiga-ui/core';

@Component({
  selector: 'app-collection-list',
  imports: [SkeletonComponent, CheckboxComponent, FormsModule, TuiCell, TuiLabel, TuiScrollbar, TuiTitle, TranslocoPipe, BadgeComponent, CardComponent],
  templateUrl: './collection-list.component.html',
  styleUrl: './collection-list.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CollectionListComponent {
  readonly store = inject(CollectionStore);
  private readonly osuDb = inject(OsuDbStore);
  /** The sets of each collection come from osu!.db: both are needed. */
  readonly loading = computed(() => this.store.isLoading() || (this.store.isLoaded() && this.osuDb.isLoading()));
}
