import { DecimalPipe } from '@angular/common';
import { ChangeDetectionStrategy, Component, computed, input, signal } from '@angular/core';
import { OsuBeatmapSet, setStatus } from '@entities/osu-db/osu-db.model';
import { openUrl } from '@tauri-apps/plugin-opener';
import { modeColor, modeIcon, starBg, starFg } from '@shared/utils';
import { TuiExpand, TuiIcon, TuiLink } from '@taiga-ui/core';
import { TuiBadge } from '@taiga-ui/kit';

/** Pips of each mode on the closed card; the rest are only counted. */
const MAX_PIPS = 6;

@Component({
  selector: 'app-beatmap-set-card',
  imports: [DecimalPipe, TuiBadge, TuiExpand, TuiIcon, TuiLink],
  templateUrl: './beatmap-set-card.component.html',
  styleUrl: './beatmap-set-card.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BeatmapSetCardComponent {
  readonly set = input.required<OsuBeatmapSet>();

  readonly expanded = signal(false);
  readonly status = computed(() => setStatus(this.set()));

  /** By mode, then by stars. */
  readonly sortedDiffs = computed(() =>
    [...this.set().diffs].sort((a, b) => a.mode !== b.mode ? a.mode - b.mode : a.stars - b.stars)
  );

  /** The pips of each mode: its easiest diffs, and how many more it has. */
  readonly pips = computed(() => {
    const diffs = this.sortedDiffs();
    return [...new Set(diffs.map(d => d.mode))].map(mode => {
      const ofMode = diffs.filter(d => d.mode === mode);
      return { mode, diffs: ofMode.slice(0, MAX_PIPS), more: Math.max(0, ofMode.length - MAX_PIPS) };
    });
  });

  toggleExpand(event: Event) {
    event.stopPropagation();
    this.expanded.update(v => !v);
  }

  openSet(event: Event) {
    event.stopPropagation();
    const id = this.set().beatmapsetId;
    if (id <= 0) return;
    openUrl(`https://osu.ppy.sh/beatmapsets/${id}`);
  }

  readonly diffBg = starBg;
  readonly diffFg = starFg;
  readonly modeIcon = modeIcon;
  readonly modeColor = modeColor;
}
