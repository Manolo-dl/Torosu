import { Component, effect, inject, resource, Signal, signal } from '@angular/core';
import { form, required, validateAsync, FormField, debounce } from '@angular/forms/signals';
import { invoke } from '@tauri-apps/api/core';
import { StatefulInputComponent } from "@shared/ui";
import { PackStore } from '@entities/pack';

@Component({
  selector: 'app-pack-details-form',
  imports: [StatefulInputComponent, FormField],
  templateUrl: './pack-details-form.component.html',
  styleUrl: './pack-details-form.component.css',
})
export class PackDetailsFormComponent {

  readonly store = inject(PackStore);

  private readonly model = signal({ title: '', finalCreator: '' });
  readonly detailsForm = form(
    this.model,
    (schemaPath) => {
      required(schemaPath.finalCreator, { message: 'Creator name is required' });
      required(schemaPath.title, { message: 'Pack title is required' });
      validateAsync(schemaPath.title, {
        params: ({ value }) => value(),
        factory: (titleSignal: Signal<string | undefined>) =>
          resource({
            params: () => titleSignal(),
            loader: async ({ params }) => {
              if (!params || params.trim().length === 0) return true;
              return invoke<boolean>('validate_pack_folder', { folderName: params });
            },
          }),
        onSuccess: (isAvailable) =>
          isAvailable ? null : { kind: 'folderExists', message: 'A pack with this title already exists' },
        onError: (error) => ({ kind: 'validationError', message: `Validation failed: ${error}` })
      });
      debounce(schemaPath.title, 500);
    }
  );

  constructor() {
    effect(() => {
      const titleField = this.detailsForm.title();
      if (titleField.valid()) {
        this.store.setTitle(titleField.value());
      }
      this.store.setTitleValid(titleField.valid() && !titleField.pending());
    });

    effect(() => {
      if (this.detailsForm.finalCreator().valid()) {
        this.store.setFinalCreator(this.detailsForm.finalCreator().value());
      }
    });

    effect(() => {
      if (this.store.title() === '' && this.store.finalCreator() === '') {
        this.detailsForm().reset({ title: '', finalCreator: '' });
      }
    });
  }
}
