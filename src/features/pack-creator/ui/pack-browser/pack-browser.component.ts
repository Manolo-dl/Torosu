import { ButtonComponent, BadgeComponent, CardComponent, IconComponent } from '@shared/ui';
import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import type { OsuBeatmapSet, OsuDiff } from '@entities/osu-db';
import { PackStore } from '../../pack.store';
import { BeatmapSetListComponent } from '@entities/osu-db/ui';
import { TuiTitle } from '@taiga-ui/core';

@Component({
  selector: 'app-pack-browser',
  imports: [BeatmapSetListComponent, TuiTitle, TranslocoPipe, ButtonComponent, BadgeComponent, CardComponent, IconComponent],
  templateUrl: './pack-browser.component.html',
  styleUrl: './pack-browser.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PackBrowserComponent {
  readonly store = inject(PackStore);

  addDiff(set: OsuBeatmapSet, diff: OsuDiff, event: Event) {
    // Not a click on the card, which would open or close it.
    event.stopPropagation();
    this.store.addDiff(set, diff);
  }
}
