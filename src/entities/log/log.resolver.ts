import { inject } from '@angular/core';
import type { ResolveFn } from '@angular/router';
import { LogStore } from './log.store';
import { TuiNotificationService } from '@taiga-ui/core';

export const logResolver:ResolveFn<void> = async () => {

  const store = inject(LogStore);
  const alerts = inject(TuiNotificationService);

  if (!store.isLoaded()) {
    alerts.open('Loading logs, please wait...', { appearance: 'info' }).subscribe();
    await store.load();
  }
};
