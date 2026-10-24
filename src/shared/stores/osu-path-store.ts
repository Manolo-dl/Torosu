import { patchState, signalStore, withComputed, withMethods, withState } from '@ngrx/signals';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { TuiNotificationService } from '@taiga-ui/core';
import { computed, inject } from '@angular/core';

interface OsuPathState {
  path: string | null;
  error: string | null;
  isLoading: boolean;
}

export const OsuPathStore = signalStore(
  { providedIn: 'root' },
  withState<OsuPathState>({ path: null, error: null, isLoading: false }),

  withComputed((store) => ({
    /** Detection ran and found no osu! folder: the app asks for one. */
    missing: computed(() => !store.path() && store.error() !== null),
  })),

  withMethods((store, alerts = inject(TuiNotificationService)) => {
    let detectPromise: Promise<void> | undefined;

    /** Finds the folder (saved, from the running osu!, or the usual places). Quiet
     * unless `announce`: the folder is found on every start. */
    async function doDetect(announce: boolean) {
      patchState(store, { isLoading: true });
      try {
        const path = await invoke<string>('get_osu_path');
        await invoke('save_osu_path', { path });
        patchState(store, { path, error: null });
        if (announce) alerts.open('osu! folder found', { appearance: 'positive' }).subscribe();
      } catch (error) {
        if (announce) alerts.open('osu! folder not found', { appearance: 'negative' }).subscribe();
        patchState(store, { error: String(error) });
        detectPromise = undefined;
      } finally {
        patchState(store, { isLoading: false });
      }
    }

    /** Checks the folder and uses it from now on; throws if it is not osu!'s. */
    async function setPath(path: string) {
      await invoke('save_osu_path', { path });
      patchState(store, { path, error: null });
      detectPromise = Promise.resolve();
    }

    return {
      async detect(): Promise<void> {
        if (!detectPromise) {
          detectPromise = doDetect(false);
        }
        return detectPromise;
      },

      /** Detection asked for by the user. */
      async retry(): Promise<void> {
        detectPromise = doDetect(true);
        return detectPromise;
      },

      setPath,

      async selectManually() {
        const selected = await open({
          directory: true,
          multiple: false,
          title: 'Select your osu! folder',
        });
        if (!selected) return;

        try {
          await setPath(selected as string);
          alerts.open('osu! folder saved', { appearance: 'positive' }).subscribe();
        } catch (e) {
          alerts.open(String(e), { appearance: 'negative' }).subscribe();
        }
      },
    };
  }),
);
