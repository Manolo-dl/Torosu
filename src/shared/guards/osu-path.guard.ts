import { inject } from '@angular/core';
import type { CanActivateFn } from '@angular/router';
import { Router } from '@angular/router';
import { OsuPathStore } from '@shared/stores'; 

export const osuPathGuard: CanActivateFn = async () => {
  const osuPath = inject(OsuPathStore);
  const router = inject(Router);

  await osuPath.detect();

  if (!osuPath.path()) {
    return router.createUrlTree(['']);
  }

  return true;
};
