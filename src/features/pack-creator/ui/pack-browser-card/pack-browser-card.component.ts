import { DecimalPipe } from '@angular/common';
import { ChangeDetectionStrategy, Component, computed, inject, input, signal } from '@angular/core';
import { OsuBeatmapSet, OsuDiff, setStatus } from '@entities/osu-db';
import { PackStore } from '@entities/pack';
import { starBg, starFg } from '@shared/utils';
import { TuiButton, TuiExpand, TuiIcon } from '@taiga-ui/core';
import { TuiBadge } from '@taiga-ui/kit';

@Component({
  selector: 'app-pack-browser-card',
  imports: [DecimalPipe, TuiBadge, TuiButton, TuiExpand, TuiIcon],
  templateUrl: './pack-browser-card.component.html',
  styleUrl: './pack-browser-card.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PackBrowserCardComponent {

  readonly set = input.required<OsuBeatmapSet>();
  readonly store = inject(PackStore);

  readonly expanded = signal(false);
  readonly status = computed(() => setStatus(this.set()));
  readonly starBg = starBg;
  readonly starFg = starFg;

  toggleExpand(event: Event) {
    event.stopPropagation();
    this.expanded.update(v => !v);
  }

  addDiff(diff: OsuDiff, event: Event) {
    event.stopPropagation();
    const set = this.set();
    const newDiffName = `[${diff.creator}] ${set.artist} - ${set.title} (${diff.diffName})`;

    this.store.toggleDiff({
      md5: diff.md5,
      folderName: set.folderName,
      fileName: diff.fileName,
      audio: diff.audio,
      newDiffName: newDiffName,
    });
  }
}
