import { inject } from '@angular/core';
import type { ResolveFn } from '@angular/router';
import { OsuReaderStore } from './osu-reader.store';

export const osuReaderResolver: ResolveFn<void> = () => {
  inject(OsuReaderStore).connect();
};
