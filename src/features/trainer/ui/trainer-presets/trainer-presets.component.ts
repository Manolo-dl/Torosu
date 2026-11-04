import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { ConfigurationStore } from '@entities/configuration';
import { NotifyService } from '@shared/services';
import { PlatformStore } from '@shared/stores';
import { FormsModule } from '@angular/forms';
import { TuiButton, TuiInput, TuiLabel, TuiTitle } from '@taiga-ui/core';
import { TuiCardLarge } from '@taiga-ui/layout';
import { BUILT_IN_PRESETS, TrainerPreset } from '../../models/trainer.model';
import { isSafeShortcut, shortcutLabel, shortcutOf } from '../../models/shortcut';
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
  private readonly config = inject(ConfigurationStore);
  private readonly notify = inject(NotifyService);
  readonly platform = inject(PlatformStore);
  readonly builtIn = BUILT_IN_PRESETS;

  /** The keyboard shortcuts are on in the settings (off by default). */
  readonly shortcutsOn = computed(() => this.config.config()?.trainer.shortcuts_enabled ?? false);
  readonly allPresets = computed(() => [...this.builtIn, ...this.presets.custom()]);
  readonly label = shortcutLabel;

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

  /**
   * Sets the shortcut of `preset` to the keys pressed in its field: Backspace
   * or Delete alone take it away, Escape leaves the field, Tab goes on.
   */
  async capture(event: KeyboardEvent, preset: string) {
    if (event.key === 'Tab') return;
    event.preventDefault();
    const plain = !event.ctrlKey && !event.altKey && !event.shiftKey && !event.metaKey;
    if (event.key === 'Escape' && plain) {
      (event.target as HTMLElement).blur();
      return;
    }
    if ((event.key === 'Backspace' || event.key === 'Delete') && plain) {
      await this.presets.setShortcut(preset, '');
      return;
    }

    const keys = shortcutOf(event);
    if (!keys) return;
    if (!isSafeShortcut(keys)) {
      this.notify.info('Add Ctrl, Alt, Shift or Super (or use a function key), so osu! keeps its keys.');
      return;
    }
    const taken = this.config.config()?.trainer.shortcuts?.find(s => s.keys === keys && s.preset !== preset);
    if (taken) {
      this.notify.error(`${shortcutLabel(keys)} already runs "${taken.preset}".`);
      return;
    }
    await this.presets.setShortcut(preset, keys);
  }

  /** What applying it gives, e.g. `4 converts`. */
  describe(preset: TrainerPreset): string {
    const count = preset.converts.length;
    return count === 1 ? '1 convert' : `${count} converts`;
  }
}
