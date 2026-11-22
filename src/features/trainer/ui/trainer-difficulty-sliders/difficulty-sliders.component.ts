import { ButtonComponent, CardComponent, TooltipDirective } from '@shared/ui';
import { TranslocoPipe } from '@jsverse/transloco';
import { DecimalPipe } from '@angular/common';
import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { OsuReaderStore } from '@entities/osu-reader';
import { DifficultyKey } from '../../models/trainer.model';
import { TrainerStore } from '../../stores/trainer.store';
import { TuiSlider, TuiTitle } from '@taiga-ui/core';

@Component({
  selector: 'app-trainer-difficulty-sliders',
  imports: [DecimalPipe, FormsModule, TuiSlider, TuiTitle, TranslocoPipe, ButtonComponent, CardComponent, TooltipDirective],
  templateUrl: './difficulty-sliders.component.html',
  styleUrl: './difficulty-sliders.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TrainerDifficultySlidersComponent {
  readonly trainer = inject(TrainerStore);
  private readonly reader = inject(OsuReaderStore);
  readonly keys: DifficultyKey[] = ['hp', 'cs', 'ar', 'od'];

  /** The selected map's own values, to show how far each slider moved; `null` with no map. */
  readonly originals = computed((): Record<DifficultyKey, number> | null => {
    const beatmap = this.reader.data()?.beatmap;
    return beatmap ? { hp: beatmap.hp, cs: beatmap.cs, ar: beatmap.ar, od: beatmap.od } : null;
  });

  /** How many unlocked stats are not the map's. */
  readonly changed = computed(() => {
    const originals = this.originals();
    if (!originals) return 0;
    return this.keys.filter(key => !this.trainer[key]().locked && this.trainer[key]().value !== originals[key]).length;
  });

  /** Back to the map's values, but for the locked stats. */
  restoreAll() {
    const originals = this.originals();
    if (!originals) return;
    for (const key of this.keys) {
      if (!this.trainer[key]().locked) this.trainer.setValue(key, originals[key]);
    }
  }
}
