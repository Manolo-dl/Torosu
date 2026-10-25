import { inject } from '@angular/core';
import type { ResolveFn } from '@angular/router';
import { CollectionStore } from './collection.store';
import { TuiNotificationService } from '@taiga-ui/core';

export const collectionResolver: ResolveFn<void> = async () => {

  const store = inject(CollectionStore);
  const alerts = inject(TuiNotificationService);

  if (!store.isLoaded()) {
    alerts.open('Loading collections, please wait...', { appearance: 'info' }).subscribe();
    await store.load();
  }
};
