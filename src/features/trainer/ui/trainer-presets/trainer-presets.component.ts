import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { TuiButton, TuiInput, TuiLabel, TuiTitle } from '@taiga-ui/core';
import { TuiCardLarge } from '@taiga-ui/layout';
import { BUILT_IN_PRESETS, TrainerPreset } from '../../models/trainer.model';
import { TrainerPresetsStore } from '../../stores/trainer-presets.store';
import { TrainerStore } from '../../stores/trainer.store';

/** Presets that replace the converts, and saving the current ones as one. */
@Component({
  selector: 'app-trainer-presets',
  imports: [FormsModule, TuiButton, TuiCardLarge, TuiInput, TuiLabel, TuiTitle],
  templateUrl: './trainer-presets.component.html',
  styleUrl: './trainer-presets.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TrainerPresetsComponent {
  readonly presets = inject(TrainerPresetsStore);
  private readonly trainer = inject(TrainerStore);
  readonly builtIn = BUILT_IN_PRESETS;

  /** Whether the name of a new preset is being typed. */
  readonly naming = signal(false);
  readonly name = signal('');

  /** Converts without tasks are no preset. */
  readonly canSave = computed(() => this.trainer.converts().some(c => c.tasks.length > 0));

  async save() {
    const name = this.name().trim();
    if (!name) return;
    await this.presets.saveCurrent(name);
    this.naming.set(false);
    this.name.set('');
  }

  /** What applying it gives, e.g. `4 converts`. */
  describe(preset: TrainerPreset): string {
    const count = preset.converts.length;
    return count === 1 ? '1 convert' : `${count} converts`;
  }
}
