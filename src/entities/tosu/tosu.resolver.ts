import { inject } from '@angular/core';
import type { ResolveFn } from '@angular/router';
import { TosuStore } from './tosu.store';
import { TuiNotificationService } from '@taiga-ui/core';

export const tosuResolver: ResolveFn<void> = () => {

  const tosuStore = inject(TosuStore);
  const alerts = inject(TuiNotificationService);
 
  if (!tosuStore.connected()) {
    tosuStore.connect();
    alerts.open('Connecting to Tosu, please wait...', { appearance: 'info' }).subscribe();
  }
};
