import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import type { OsuBeatmapSet } from '@entities/osu-db';
import { PackStore } from '@entities/pack';
import { ScrollingModule } from '@angular/cdk/scrolling';
import { TuiScrollControls, TuiScrollRef, TuiTitle } from '@taiga-ui/core';
import { TuiBadge } from '@taiga-ui/kit';
import { TuiCardLarge } from '@taiga-ui/layout';
import { PackBrowserCardComponent } from '../pack-browser-card/pack-browser-card.component';

@Component({
  selector: 'app-pack-browser',
  imports: [ScrollingModule, TuiScrollRef, TuiScrollControls, TuiCardLarge, TuiTitle, TuiBadge, PackBrowserCardComponent],
  templateUrl: './pack-browser.component.html',
  styleUrl: './pack-browser.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PackBrowserComponent {

  readonly store = inject(PackStore);

  trackSet(_index: number, set: OsuBeatmapSet): string {
    return set.folderName;
  }
}
