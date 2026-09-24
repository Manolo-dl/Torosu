import { inject } from '@angular/core';
import type { ResolveFn } from '@angular/router';
import { ToastStore } from '@shared/stores';
import { DatabaseStore } from './database.store';

export const databaseResolver: ResolveFn<void> = async () => {
  
  const toast = inject(ToastStore);
  const databaseStore = inject(DatabaseStore);

  if (!databaseStore.isLoaded()) {
    toast.show('info', 'Loading database...');
    await databaseStore.load();
  }
};
