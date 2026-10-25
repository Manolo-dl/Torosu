import { inject } from '@angular/core';
import type { ResolveFn } from '@angular/router';
import { OsuDbStore } from './osu-db.store';
import { TuiNotificationService } from '@taiga-ui/core';

export const osuDbResolver: ResolveFn<void> = async () => {
  
  const store = inject(OsuDbStore);
  const alerts = inject(TuiNotificationService);

  if (!store.isLoaded()) {
    alerts.open('Loading osu!.db, please wait...', { appearance: 'info' }).subscribe();
    await store.load();
  }
};
