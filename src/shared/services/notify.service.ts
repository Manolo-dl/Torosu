import { Injectable, inject } from '@angular/core';
import { TuiNotificationService } from '@taiga-ui/core';

/**
 * The app's notifications. How long they stay and their close button are
 * set for all of them in `app.config.ts`.
 */
@Injectable({ providedIn: 'root' })
export class NotifyService {
  private readonly notifications = inject(TuiNotificationService);

  success(message: string) {
    this.show(message, 'positive');
  }

  error(message: string) {
    this.show(message, 'negative');
  }

  info(message: string) {
    this.show(message, 'info');
  }

  private show(message: string, appearance: string) {
    this.notifications.open(message, { appearance }).subscribe();
  }
}
