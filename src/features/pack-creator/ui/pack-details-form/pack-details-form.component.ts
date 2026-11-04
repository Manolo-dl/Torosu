import { TranslocoPipe } from '@jsverse/transloco';
import { Component, effect, inject, resource, Signal, signal } from '@angular/core';
import { form, required, validateAsync, FormField, debounce } from '@angular/forms/signals';
import { invoke } from '@tauri-apps/api/core';
import { StatefulInputComponent } from "@shared/ui";
import { PackStore } from '../../pack.store';
import { TuiTitle } from '@taiga-ui/core';
import { TuiCardLarge } from '@taiga-ui/layout';

@Component({
  selector: 'app-pack-details-form',
  imports: [StatefulInputComponent, FormField, TuiCardLarge, TuiTitle, TranslocoPipe],
  templateUrl: './pack-details-form.component.html',
  styleUrl: './pack-details-form.component.less',
})
export class PackDetailsFormComponent {

  readonly store = inject(PackStore);

  private readonly model = signal({ title: '', finalCreator: '' });
  readonly detailsForm = form(
    this.model,
    (schemaPath) => {
      required(schemaPath.finalCreator, { message: 'packs.details.creatorRequired' });
      required(schemaPath.title, { message: 'packs.details.titleRequired' });
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
          isAvailable ? null : { kind: 'folderExists', message: 'packs.details.titleTaken' },
        onError: (error) => {
          console.error('validate_pack_folder failed', error);
          return { kind: 'validationError', message: 'packs.details.validationFailed' };
        },
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
