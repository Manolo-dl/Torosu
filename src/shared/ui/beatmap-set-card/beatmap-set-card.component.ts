import { DecimalPipe } from '@angular/common';
import { ChangeDetectionStrategy, Component, Input, signal } from '@angular/core';
import { OsuBeatmapSet, OsuDiff, setStatus } from '@entities/osu-db/osu-db.model';
import { openUrl } from '@tauri-apps/plugin-opener';
import { modeColor, modeIcon, starBg, starFg } from '@shared/utils';
import { TuiExpand, TuiIcon, TuiLink } from '@taiga-ui/core';
import { TuiBadge } from '@taiga-ui/kit';

@Component({
  selector: 'app-beatmap-set-card',
  imports: [DecimalPipe, TuiBadge, TuiExpand, TuiIcon, TuiLink],
  templateUrl: './beatmap-set-card.component.html',
  styleUrl: './beatmap-set-card.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BeatmapSetCardComponent {
  @Input({ required: true }) set!: OsuBeatmapSet;

  readonly expanded = signal(false);

  toggleExpand(event: Event) {
    event.stopPropagation();
    this.expanded.update(v => !v);
  }

  get status() {
    return setStatus(this.set);
  }

  openSet(event: Event) {
    event.stopPropagation();
    if (this.set.beatmapsetId <= 0) return;
    openUrl(`https://osu.ppy.sh/beatmapsets/${this.set.beatmapsetId}`);
  }

  sortedDiffs(diffs: OsuDiff[]): OsuDiff[] {
    return [...diffs].sort((a, b) => a.mode !== b.mode ? a.mode - b.mode : a.stars - b.stars);
  }

  uniqueModes(diffs: OsuDiff[]): number[] {
    return [...new Set(diffs.map(d => d.mode))].sort();
  }

  visibleDiffsForMode(diffs: OsuDiff[], mode: number): OsuDiff[] {
    return diffs.filter(d => d.mode === mode).sort((a, b) => a.stars - b.stars).slice(0, 6);
  }

  extraDiffsForMode(diffs: OsuDiff[], mode: number): number {
    return Math.max(0, diffs.filter(d => d.mode === mode).length - 6);
  }

  readonly diffBg = starBg;
  readonly diffFg = starFg;
  readonly modeIcon = modeIcon;
  readonly modeColor = modeColor;
}
