import { inject } from '@angular/core';
import type { ResolveFn } from '@angular/router';
import { CollectionStore } from './collection.store';
import { ToastStore } from '@shared/stores';

export const collectionResolver: ResolveFn<void> = async () => {

  const store = inject(CollectionStore);
  const toast = inject(ToastStore);

  if (!store.isLoaded()) {
    toast.show('info', 'Loading collections, please wait...');
    await store.load();
  }
};
