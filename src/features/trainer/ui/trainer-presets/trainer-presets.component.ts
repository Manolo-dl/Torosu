import { ChangeDetectionStrategy, Component, computed, DestroyRef, inject, signal } from '@angular/core';
import { ConfigurationStore } from '@entities/configuration';
import { PlatformStore } from '@shared/stores';
import { FormsModule } from '@angular/forms';
import { TuiButton, TuiIcon, TuiInput, TuiLabel, TuiTitle } from '@taiga-ui/core';
import { TuiCardLarge } from '@taiga-ui/layout';
import { BUILT_IN_PRESETS, TrainerPreset } from '../../models/trainer.model';
import { isSafeShortcut, shortcutLabel, shortcutOf } from '../../models/shortcut';
import { TrainerPresetsStore } from '../../stores/trainer-presets.store';
import { TrainerStore } from '../../stores/trainer.store';
import { TrainerShortcutsService } from '../../services/trainer-shortcuts.service';

/** Presets that replace the converts, and saving the current ones as one. */
@Component({
  selector: 'app-trainer-presets',
  imports: [FormsModule, TuiButton, TuiCardLarge, TuiIcon, TuiInput, TuiLabel, TuiTitle],
  templateUrl: './trainer-presets.component.html',
  styleUrl: './trainer-presets.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TrainerPresetsComponent {
  readonly presets = inject(TrainerPresetsStore);
  private readonly trainer = inject(TrainerStore);
  private readonly config = inject(ConfigurationStore);
  readonly platform = inject(PlatformStore);
  readonly shortcuts = inject(TrainerShortcutsService);

  /** The preset whose shortcut field is taking keys, and what is wrong with the last ones pressed. */
  readonly editing = signal<string | null>(null);
  readonly problem = signal<{ preset: string; message: string } | null>(null);

  constructor() {
    // Leaving the page from a field may skip its blur: the shortcuts come back anyway.
    inject(DestroyRef).onDestroy(() => this.shortcuts.paused.set(false));
  }
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
    this.problem.set(null);
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
      this.problem.set({ preset, message: `${shortcutLabel(keys)} alone is a key of the game: add Ctrl, Alt, Shift or Super.` });
      return;
    }
    const taken = this.config.config()?.trainer.shortcuts?.find(s => s.keys === keys && s.preset !== preset);
    if (taken) {
      this.problem.set({ preset, message: `${shortcutLabel(keys)} already runs "${taken.preset}".` });
      return;
    }
    await this.presets.setShortcut(preset, keys);
  }

  /** While a field takes keys the shortcuts are let go, or the system would keep their keys from it. */
  startEditing(preset: string) {
    this.editing.set(preset);
    this.problem.set(null);
    this.shortcuts.paused.set(true);
  }

  stopEditing() {
    this.editing.set(null);
    this.shortcuts.paused.set(false);
  }

  /** How the shortcut of `preset` stands: none set, working, or not taken by the system. */
  stateOf(preset: string): 'unset' | 'active' | 'failed' | 'pending' {
    const keys = this.presets.shortcutOf(preset);
    if (!keys) return 'unset';
    return this.shortcuts.states()[keys] ?? 'pending';
  }

  /** What applying it gives, e.g. `4 converts`. */
  describe(preset: TrainerPreset): string {
    const count = preset.converts.length;
    return count === 1 ? '1 convert' : `${count} converts`;
  }
}
