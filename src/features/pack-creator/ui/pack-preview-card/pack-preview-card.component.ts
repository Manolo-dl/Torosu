import { Component, effect, inject, input, signal } from '@angular/core';
import { form, required, FormField, validate } from '@angular/forms/signals';
import { PackStore, SelectedDiff } from '@entities/pack';
import { StatefulInputComponent } from "@shared/ui";

@Component({
  selector: 'app-pack-preview-card',
  imports: [FormField, StatefulInputComponent],
  templateUrl: './pack-preview-card.component.html',
  styleUrl: './pack-preview-card.component.css',
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

  constructor() {
    effect(() => {
      this.model.set({ newDiffName: this.diff().newDiffName});
    });

    effect(() => {
      this.store.updateDiffName(this.diff(), this.nameForm.newDiffName().value());
    });
  }

  onRemove() {
    this.store.toggleDiff(this.diff());
  }

  isDuplicate(): boolean {
    const result = this.store.duplicateNames().has(this.diff().md5);
    return result;
}
}
