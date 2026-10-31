import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import type { OsuBeatmapSet, OsuDiff } from '@entities/osu-db';
import { selectedDiff } from '../../pack.model';
import { PackStore } from '../../pack.store';
import { BeatmapSetListComponent } from '@shared/ui';
import { TuiButton, TuiIcon, TuiTitle } from '@taiga-ui/core';
import { TuiBadge } from '@taiga-ui/kit';
import { TuiCardLarge } from '@taiga-ui/layout';

@Component({
  selector: 'app-pack-browser',
  imports: [BeatmapSetListComponent, TuiBadge, TuiButton, TuiCardLarge, TuiIcon, TuiTitle],
  templateUrl: './pack-browser.component.html',
  styleUrl: './pack-browser.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PackBrowserComponent {
  readonly store = inject(PackStore);

  addDiff(set: OsuBeatmapSet, diff: OsuDiff, event: Event) {
    // Not a click on the card, which would open or close it.
    event.stopPropagation();
    this.store.toggleDiff(selectedDiff(set, diff));
  }
}
