import { inject } from '@angular/core';
import type { ResolveFn } from '@angular/router';
import { LogStore } from './log.store';
import { ToastStore } from '@shared/stores';

export const logResolver:ResolveFn<void> = async () => {

  const store = inject(LogStore);
  const toast = inject(ToastStore);

  if (!store.isLoaded()) {
    toast.show('info', 'Loading logs, please wait...');
    await store.load();
  }
};
