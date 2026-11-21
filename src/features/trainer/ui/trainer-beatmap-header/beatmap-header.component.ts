import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { ConfigurationStore } from '@entities/configuration';
import { OsuReaderStore, SkillSummaryComponent } from '@entities/osu-reader';
import { StarBadgeComponent, ButtonComponent, IconComponent } from '@shared/ui';
import { GAME_MODES, modeColor, modeIcon } from '@shared/utils';
import { TuiTitle } from '@taiga-ui/core';

@Component({
  selector: 'app-trainer-beatmap-header',
  imports: [SkillSummaryComponent, StarBadgeComponent, TuiTitle, TranslocoPipe, ButtonComponent, IconComponent],
  templateUrl: './beatmap-header.component.html',
  styleUrl: './beatmap-header.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TrainerBeatmapHeaderComponent {
  readonly reader = inject(OsuReaderStore);
  private readonly config = inject(ConfigurationStore);
  readonly skillDisplay = computed(() => this.config.config()?.trainer.skill_display ?? 'dan');
  /**
   * With `include_mods` on, the skill with the mods selected in osu! when
   * they change it; the map's own otherwise. Only one is shown.
   */
  readonly played = computed(() => (this.config.config()?.trainer.include_mods ? this.reader.played() : null));
  readonly shownSkill = computed(() => this.played()?.skill ?? this.reader.skill());
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
