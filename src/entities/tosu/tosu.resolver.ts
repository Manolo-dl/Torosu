import { inject } from '@angular/core';
import type { ResolveFn } from '@angular/router';
import { TosuStore } from './tosu.store';
import { ToastStore } from '@shared/stores';

export const tosuResolver: ResolveFn<void> = () => {

  const tosuStore = inject(TosuStore);
  const toast = inject(ToastStore);
 
  if (!tosuStore.connected()) {
    tosuStore.connect();
    toast.show('info', 'Connecting to Tosu, please wait...');
  }
};
