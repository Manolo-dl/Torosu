import { CardComponent, IconComponent, StarBadgeComponent, TooltipDirective } from '@shared/ui';
import { CountUpDirective } from '@shared/directives';
import { OsuDbStore } from '@entities/osu-db';
import { TranslocoPipe } from '@jsverse/transloco';
import { afterRenderEffect, ChangeDetectionStrategy, Component, computed, inject, viewChild } from '@angular/core';
import { CdkVirtualScrollViewport, ScrollingModule } from '@angular/cdk/scrolling';
import { SelectedDiff } from '../../pack.model';
import { PackStore } from '../../pack.store';
import { TuiScrollControls, TuiScrollRef, TuiTitle } from '@taiga-ui/core';
import { PackPreviewCardComponent } from "../pack-preview-card/pack-preview-card.component";

@Component({
  selector: 'app-pack-preview',
  imports: [ScrollingModule, PackPreviewCardComponent, TuiScrollControls, TuiScrollRef, TuiTitle, TranslocoPipe, CardComponent, CountUpDirective, IconComponent, StarBadgeComponent, TooltipDirective],
  templateUrl: './pack-preview.component.html',
  styleUrl: './pack-preview.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PackPreviewComponent {

  readonly store = inject(PackStore);
  private readonly osuDb = inject(OsuDbStore);
  private readonly viewport = viewChild(CdkVirtualScrollViewport);

  constructor() {
    // The list is as tall as its diffs: told to the virtual scroll, which
    // otherwise keeps rendering for the height it measured first.
    afterRenderEffect(() => {
      this.store.selectedDiffs().length;
      this.viewport()?.checkViewportSize();
    });
  }

  /** The easiest and the hardest diff picked, in stars. */
  readonly stars = computed(() => {
    const byMd5 = this.osuDb.diffsByMd5();
    const stars = this.store.selectedDiffs().map(d => byMd5.get(d.md5)?.stars).filter((s): s is number => s !== undefined);
    return stars.length ? { low: Math.min(...stars), high: Math.max(...stars) } : null;
  });

  trackDiff(_index: number, diff: SelectedDiff): string {
    return diff.md5;
  }
}
