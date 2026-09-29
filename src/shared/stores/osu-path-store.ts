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
    selectLabel: computed(() => store.error() ? 'Select manually' : 'Change path'),
    osuPathLabel: computed(() => store.path() ? store.path() : 'osu! path not detected'),
  })),

  withMethods((store, alerts = inject(TuiNotificationService)) => {
    let detectPromise: Promise<void> | undefined;

    async function doDetect() {
      patchState(store, { isLoading: true });
      try {
        const path = await invoke<string>('get_osu_path');
        alerts.open('Osu! path detected', { appearance: 'positive' }).subscribe();

        patchState(store, { path, error: null });
        await invoke('save_osu_path', { path });
        alerts.open('Osu! path saved', { appearance: 'positive' }).subscribe();
      } catch (error) {
        alerts.open('Failed to detect Osu! path', { appearance: 'negative' }).subscribe();
        patchState(store, { error: error as string });
        detectPromise = undefined;
      } finally {
        patchState(store, { isLoading: false });
      }
    }

    return {
      async detect(): Promise<void> {
        if (!detectPromise) {
          detectPromise = doDetect();
        }
        return detectPromise;
      },

      async selectManually() {
        const selected = await open({
          directory: true,
          multiple: false,
          title: 'Select your osu! folder',
        });

        if (selected) {
          patchState(store, { path: selected as string, error: null });
          detectPromise = Promise.resolve();
          try {
            await invoke('save_osu_path', { path: selected });
            alerts.open('Osu! path saved', { appearance: 'positive' }).subscribe();
          } catch (e) {
            alerts.open('Failed to save osu! path', { appearance: 'negative' }).subscribe();
            console.error('Failed to save osu! path:', e);
          }
        }
      },
    };
  }),
);
