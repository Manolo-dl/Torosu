import { ChangeDetectionStrategy, Component, effect, input, model, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { TuiButton, TuiInput, TuiLabel } from '@taiga-ui/core';
import { invoke } from '@tauri-apps/api/core';

export interface NameFormatPreset {
  label: string;
  format: string;
}

export interface NameFormatField {
  field: string;
  meaning: string;
}

/**
 * A name format (see `name_format.rs`): the text, formats to start from, what
 * it gives for a sample and the placeholders it can use.
 */
@Component({
  selector: 'app-name-format-field',
  imports: [FormsModule, TuiButton, TuiInput, TuiLabel],
  templateUrl: './name-format-field.component.html',
  styleUrl: './name-format-field.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NameFormatFieldComponent {
  readonly format = model.required<string>();
  readonly presets = input.required<NameFormatPreset[]>();
  readonly fields = input.required<NameFormatField[]>();
  /** Backend command that names a sample with a `format`, as it names the real ones. */
  readonly exampleCommand = input.required<string>();

  readonly example = signal('');

  constructor() {
    effect(() => {
      const format = this.format();
      invoke<string>(this.exampleCommand(), { format })
        .then(example => {
          // A later format may have been typed meanwhile.
          if (this.format() === format) this.example.set(example);
        })
        .catch(() => this.example.set(''));
    });
  }
}
