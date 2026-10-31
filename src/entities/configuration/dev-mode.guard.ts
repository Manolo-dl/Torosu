import { inject } from '@angular/core';
import type { CanActivateFn } from '@angular/router';
import { Router } from '@angular/router';
import { ConfigurationStore } from './configuration.store';

/** The developer pages only open with developer mode on in the configuration. */
export const devModeGuard: CanActivateFn = async () => {
  const config = inject(ConfigurationStore);
  const router = inject(Router);

  await config.ensureLoaded();

  return config.config()?.dev_mode ? true : router.createUrlTree(['']);
};
