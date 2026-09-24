import { patchState, signalStore, withComputed, withMethods, withState } from '@ngrx/signals';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { ToastStore } from './toast-store';
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

  withMethods((store, toast = inject(ToastStore)) => {
    let detectPromise: Promise<void> | undefined;

    async function doDetect() {
      patchState(store, { isLoading: true });
      try {
        const path = await invoke<string>('get_osu_path');
        toast.show('success', 'Osu! path detected');

        patchState(store, { path, error: null });
        await invoke('save_osu_path', { path });
        toast.show('success', 'Osu! path saved');
      } catch (error) {
        toast.show('error', 'Failed to detect Osu! path');
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
            toast.show('success', 'Osu! path saved');
          } catch (e) {
            toast.show('error', 'Failed to save osu! path');
            console.error('Failed to save osu! path:', e);
          }
        }
      },
    };
  }),
);
