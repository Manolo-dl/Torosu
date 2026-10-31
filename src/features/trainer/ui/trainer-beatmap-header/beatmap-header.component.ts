import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { OsuReaderStore } from '@entities/osu-reader';
import { StarBadgeComponent } from '@shared/ui';
import { GAME_MODES, modeColor, modeIcon } from '@shared/utils';
import { TuiButton, TuiIcon, TuiTitle } from '@taiga-ui/core';
import { TuiButtonLoading } from '@taiga-ui/kit';

@Component({
  selector: 'app-trainer-beatmap-header',
  imports: [StarBadgeComponent, TuiButton, TuiButtonLoading, TuiIcon, TuiTitle],
  templateUrl: './beatmap-header.component.html',
  styleUrl: './beatmap-header.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TrainerBeatmapHeaderComponent {
  readonly reader = inject(OsuReaderStore);
  readonly modeIcon = modeIcon;
  readonly modeColor = modeColor;

  /** The map's own mode, as osu!'s mode number (0 osu! ... 3 mania). */
  readonly mode = computed(() => {
    const ruleset = this.reader.originalMode();
    return ruleset ? GAME_MODES.findIndex(mode => mode.ruleset === ruleset) : null;
  });
  readonly modeName = computed(() => {
    const mode = this.mode();
    return mode === null ? '' : (GAME_MODES[mode]?.name ?? '');
  });
}
