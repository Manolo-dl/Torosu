import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { form, maxLength, minLength, required, FormRoot, FormField } from '@angular/forms/signals';
import { signal } from '@angular/core';
import { UserStore } from '@entities/user';
import { ConfigurationStore } from '@entities/configuration';
import { StatefulInputComponent } from "@shared/ui";
import { TuiButton, TuiNotificationDirective, TuiTitle } from '@taiga-ui/core';
import { TuiBadge } from '@taiga-ui/kit';

@Component({
  selector: 'app-osu-cookie-input',
  imports: [FormRoot, StatefulInputComponent, FormField, TuiButton, TuiNotificationDirective, TuiTitle, TuiBadge],
  templateUrl: './osu-cookie-input.component.html',
  styleUrl: './osu-cookie-input.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class OsuCookieInputComponent {
  readonly userStore = inject(UserStore);
  readonly config = inject(ConfigurationStore);

  readonly osuCookieForm = form(
    signal({ osuSession: '' }),
    (schemaPath) => {
      required(schemaPath.osuSession, { message: 'Osu! cookie is required' });
      minLength(schemaPath.osuSession, 300, { message: 'Cookie is too short' });
      maxLength(schemaPath.osuSession, 500, { message: 'Cookie is too long' });
    },
    {
      submission: {
        action: async (field) => {
          const previous = this.userStore.user()?.osuSession ?? '';
          try {
            this.userStore.updateUser({ osuSession: field().value().osuSession });
            field().value.set({ osuSession: '' });
            field().reset();
            return null;
          } catch {
            if (previous) field().value.set({ osuSession: previous });
            return { kind: 'error', message: 'Failed to update cookie' };
          }
        },
        ignoreValidators: 'none',
      }
    }
  );

  clearSession() {
    this.userStore.clearSession();
  }
}
