import { DecimalPipe } from '@angular/common';
import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { OsuReaderStore, Ruleset } from '@entities/osu-reader';
import { modeColor, modeIcon, starBg, starFg } from '@shared/utils';
import { TuiButton, TuiIcon, TuiTitle } from '@taiga-ui/core';
import { TuiBadge, TuiButtonLoading } from '@taiga-ui/kit';

const RULESETS: Ruleset[] = ['osu', 'taiko', 'fruits', 'mania'];
const MODE_NAMES = ['osu!', 'osu!taiko', 'osu!catch', 'osu!mania'];

@Component({
  selector: 'app-trainer-beatmap-header',
  imports: [DecimalPipe, TuiBadge, TuiButton, TuiButtonLoading, TuiIcon, TuiTitle],
  templateUrl: './beatmap-header.component.html',
  styleUrl: './beatmap-header.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TrainerBeatmapHeaderComponent {
  readonly reader = inject(OsuReaderStore);
  readonly starBg = starBg;
  readonly starFg = starFg;
  readonly modeIcon = modeIcon;
  readonly modeColor = modeColor;

  /** The map's own mode, as osu!'s mode number (0 osu! ... 3 mania). */
  readonly mode = computed(() => {
    const mode = this.reader.originalMode();
    return mode ? RULESETS.indexOf(mode) : null;
  });
  readonly modeName = computed(() => {
    const mode = this.mode();
    return mode === null ? '' : MODE_NAMES[mode];
  });
}
