import { ButtonComponent, BadgeComponent, NoticeComponent, StatefulInputComponent } from '@shared/ui';
import { TranslocoPipe } from '@jsverse/transloco';
import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { form, maxLength, minLength, required, FormRoot, FormField } from '@angular/forms/signals';
import { signal } from '@angular/core';
import { UserStore } from '../../user.store';
import { ConfigurationStore } from '@entities/configuration';
import { NotifyService } from '@shared/services';
import { TranslocoService } from '@jsverse/transloco';

@Component({
  selector: 'app-osu-cookie-input',
  imports: [FormRoot, StatefulInputComponent, FormField, TranslocoPipe, ButtonComponent, BadgeComponent, NoticeComponent],
  templateUrl: './osu-cookie-input.component.html',
  styleUrl: './osu-cookie-input.component.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class OsuCookieInputComponent {
  readonly userStore = inject(UserStore);
  readonly config = inject(ConfigurationStore);
  private readonly notify = inject(NotifyService);
  private readonly transloco = inject(TranslocoService);

  /** The messages are translation keys, shown by `StatefulInputComponent`. */
  readonly osuCookieForm = form(
    signal({ osuSession: '' }),
    (schemaPath) => {
      required(schemaPath.osuSession, { message: 'downloader.cookie.isRequired' });
      minLength(schemaPath.osuSession, 300, { message: 'downloader.cookie.tooShort' });
      maxLength(schemaPath.osuSession, 500, { message: 'downloader.cookie.tooLong' });
    },
    {
      submission: {
        action: async (field) => {
          const previous = this.userStore.user()?.osuSession ?? '';
          try {
            this.userStore.updateUser({ osuSession: field().value().osuSession });
            // Emptied, not to leave the cookie on screen; the badge above says it is in use.
            field().value.set({ osuSession: '' });
            field().reset();
            this.notify.success(this.transloco.translate('downloader.cookie.updated'));
            return null;
          } catch {
            if (previous) field().value.set({ osuSession: previous });
            return { kind: 'error', message: 'downloader.cookie.failed' };
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
