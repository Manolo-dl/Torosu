import { BadgeComponent, ButtonComponent, CheckboxComponent, IconComponent, NoticeComponent, SkeletonComponent } from '@shared/ui';
import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { OsuDbStore } from '@entities/osu-db';
import { BeatmapSetCardComponent } from '@entities/osu-db/ui';
import { ImportCollectionStore } from '../../import-collections.store';

/**
 * Step 2: the file's collections, each picked or left out, with what it
 * would add (new, into one of the same name, or nothing: all there already)
 * and its sets, shown on demand.
 */
@Component({
  selector: 'app-import-preview',
  imports: [BadgeComponent, BeatmapSetCardComponent, ButtonComponent, CheckboxComponent, IconComponent, NoticeComponent, SkeletonComponent, TranslocoPipe],
  templateUrl: './import-preview.component.html',
  styleUrl: './import-preview.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ImportPreviewComponent {
  readonly store = inject(ImportCollectionStore);
  readonly osuDb = inject(OsuDbStore);

  /** Collections whose sets are shown, by name. */
  readonly expanded = signal<ReadonlySet<string>>(new Set());

  readonly allPicked = computed(() => {
    const importable = this.store.groups().filter(group => group.md5s.length > 0);
    return importable.length > 0 && importable.every(group => group.selected);
  });

  toggleExpanded(name: string) {
    const expanded = new Set(this.expanded());
    if (!expanded.delete(name)) expanded.add(name);
    this.expanded.set(expanded);
  }
}
