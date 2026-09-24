import { DecimalPipe, NgClass } from '@angular/common';
import { Component, inject, input, signal } from '@angular/core';
import { OsuBeatmapSet, OsuDiff } from '@entities/osu-db';
import { PackStore } from '@entities/pack';

@Component({
  selector: 'app-pack-browser-card',
  imports: [DecimalPipe, NgClass],
  templateUrl: './pack-browser-card.component.html',
  styleUrl: './pack-browser-card.component.css',
})
export class PackBrowserCardComponent {

  readonly set = input.required<OsuBeatmapSet>();
  readonly store = inject(PackStore);

  readonly expanded = signal(false);

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
      beatmapsetId: set.beatmapsetId,
      fileName: diff.fileName,
      audio: diff.audio,
      newDiffName: newDiffName,
    });
  }
}
