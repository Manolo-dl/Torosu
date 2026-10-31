import { ChangeDetectionStrategy, Component, computed, effect, inject, input, signal } from '@angular/core';
import { form, required, FormField } from '@angular/forms/signals';
import { SelectedDiff } from '../../pack.model';
import { PackStore } from '../../pack.store';
import { TuiButton, TuiHint, TuiIcon, TuiInput, tuiTextfieldOptionsProvider } from '@taiga-ui/core';

@Component({
  selector: 'app-pack-preview-card',
  imports: [FormField, TuiButton, TuiHint, TuiIcon, TuiInput],
  providers: [tuiTextfieldOptionsProvider({ size: signal('s'), cleaner: signal(false) })],
  templateUrl: './pack-preview-card.component.html',
  styleUrl: './pack-preview-card.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PackPreviewCardComponent {

  readonly diff = input.required<SelectedDiff>();
  readonly store = inject(PackStore);

  private readonly model = signal({ newDiffName: '' });
  readonly nameForm = form(
    this.model,
    (schemaPath) => {
      required(schemaPath.newDiffName, { message: 'Difficulty name is required' });
    }
  );

  // Shown as an icon hint instead of a line of text so every row keeps the
  // same height, which the virtual scroll in the pack preview relies on.
  readonly error = computed(() => {
    if (this.store.duplicateNames().has(this.diff().md5)) {
      return 'This name is used by another difficulty';
    }
    return this.nameForm.newDiffName().errors()[0]?.message ?? null;
  });

  constructor() {
    effect(() => {
      this.model.set({ newDiffName: this.diff().newDiffName });
    });

    effect(() => {
      this.store.updateDiffName(this.diff(), this.nameForm.newDiffName().value());
    });
  }

  onRemove() {
    this.store.toggleDiff(this.diff());
  }
}
