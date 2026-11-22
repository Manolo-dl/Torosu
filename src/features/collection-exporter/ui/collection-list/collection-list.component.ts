import { BadgeComponent, ButtonComponent, CheckboxComponent, IconComponent, NoticeComponent, SkeletonComponent } from '@shared/ui';
import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { CollectionStore } from '@entities/collection';
import { OsuDbStore } from '@entities/osu-db';
import { BeatmapSetCardComponent } from '@entities/osu-db/ui';
import { ExportCollectionsStore } from '../../export-collections.store';

/**
 * Step 1: osu!'s collections, found by name, each picked or not, with how
 * many maps and sets it has (and how many are not installed) and its sets,
 * shown on demand.
 */
@Component({
  selector: 'app-collection-list',
  imports: [BadgeComponent, BeatmapSetCardComponent, ButtonComponent, CheckboxComponent, IconComponent, NoticeComponent, SkeletonComponent, TranslocoPipe],
  templateUrl: './collection-list.component.html',
  styleUrl: './collection-list.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CollectionListComponent {
  readonly store = inject(ExportCollectionsStore);
  readonly collections = inject(CollectionStore);
  readonly osuDb = inject(OsuDbStore);

  /** The sets of each collection come from osu!.db: both are read first. */
  readonly loading = computed(() => this.collections.isLoading() || (this.collections.isLoaded() && this.osuDb.isLoading()));

  /** Whether every collection shown (with maps) is picked. */
  readonly allPicked = computed(() => {
    const shown = this.store.visibleRows().filter(row => row.md5s.length > 0);
    return shown.length > 0 && shown.every(row => row.selected);
  });

  /** Collections whose sets are shown, by name. */
  readonly expanded = signal<ReadonlySet<string>>(new Set());

  toggleExpanded(name: string) {
    const expanded = new Set(this.expanded());
    if (!expanded.delete(name)) expanded.add(name);
    this.expanded.set(expanded);
  }
}
