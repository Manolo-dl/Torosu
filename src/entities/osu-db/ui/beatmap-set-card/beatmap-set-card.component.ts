import { NgTemplateOutlet } from '@angular/common';
import { ChangeDetectionStrategy, Component, computed, effect, inject, input, signal, TemplateRef } from '@angular/core';
import { OsuBeatmapSet, OsuDiff, setStatus } from '../../osu-db.model';
import { openUrl } from '@tauri-apps/plugin-opener';
import { StarBadgeComponent, BadgeComponent, IconComponent } from '@shared/ui';
import { beatmapsetUrl, modeColor, modeIcon, starBg } from '@shared/utils';
import { BeatmapCoverService } from '../../beatmap-cover.service';
import { AppearanceStore } from '@shared/stores';
import { TuiExpand } from '@taiga-ui/core';

/** Pips of each mode on the closed card; the rest are only counted. */
const MAX_PIPS = 6;

/** What a `diffAction` template gets: the diff (`let-diff`) and its set (`let-set="set"`). */
export interface DiffActionContext {
  $implicit: OsuDiff;
  set: OsuBeatmapSet;
}

@Component({
  selector: 'app-beatmap-set-card',
  imports: [NgTemplateOutlet, StarBadgeComponent, TuiExpand, BadgeComponent, IconComponent],
  templateUrl: './beatmap-set-card.component.html',
  styleUrl: './beatmap-set-card.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BeatmapSetCardComponent {
  readonly set = input.required<OsuBeatmapSet>();
  /** Shown at the end of each diff, e.g. a button that picks it. */
  readonly diffAction = input<TemplateRef<DiffActionContext>>();

  readonly expanded = signal(false);

  /** Its cover, from its own background (`BeatmapCoverService`); none while it comes, or without one. */
  readonly cover = signal<string | null>(null);
  readonly coverLoaded = signal(false);
  private readonly covers = inject(BeatmapCoverService);
  private readonly appearance = inject(AppearanceStore);

  constructor() {
    // Again for each set: a virtual list gives a card another set as it scrolls.
    effect(onCleanup => {
      const set = this.set();
      this.appearance.covers();
      this.cover.set(null);
      this.coverLoaded.set(false);
      const file = set.diffs[0]?.fileName;
      if (!file) return;
      let current = true;
      onCleanup(() => (current = false));
      this.covers.cover(set.folderName, file).then(url => current && this.cover.set(url));
    });
  }
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
    openUrl(beatmapsetUrl(id));
  }

  readonly diffBg = starBg;
  readonly modeIcon = modeIcon;
  readonly modeColor = modeColor;
}
