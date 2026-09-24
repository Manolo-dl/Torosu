import { inject } from '@angular/core';
import type { ResolveFn } from '@angular/router';
import { OsuDbStore } from './osu-db.store';
import { ToastStore } from '@shared/stores';

export const osuDbResolver: ResolveFn<void> = async () => {
  
  const store = inject(OsuDbStore);
  const toast = inject(ToastStore);

  if (!store.isLoaded()) {
    toast.show('info', 'Loading osu!.db, please wait...');
    await store.load();
  }
};
