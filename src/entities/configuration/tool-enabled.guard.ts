import { inject } from '@angular/core';
import type { CanActivateChildFn } from '@angular/router';
import { Router } from '@angular/router';
import { ConfigurationStore } from './configuration.store';

/** Tools turned off in the configuration (`excluded_tools`) can't be opened. */
export const toolEnabledGuard: CanActivateChildFn = async (_route, state) => {
  const config = inject(ConfigurationStore);
  const router = inject(Router);

  await config.ensureLoaded();

  const excluded = config.config()?.excluded_tools ?? [];
  if (excluded.some(route => state.url === route || state.url.startsWith(`${route}/`))) {
    return router.createUrlTree(['']);
  }

  return true;
};
