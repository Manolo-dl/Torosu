import { inject } from '@angular/core';
import type { ResolveFn } from '@angular/router';
import { TuiNotificationService } from '@taiga-ui/core';
import { DatabaseStore } from './database.store';

export const databaseResolver: ResolveFn<void> = async () => {
  
  const alerts = inject(TuiNotificationService);
  const databaseStore = inject(DatabaseStore);

  if (!databaseStore.isLoaded()) {
    alerts.open('Loading database...', { appearance: 'info' }).subscribe();
    await databaseStore.load();
  }
};
