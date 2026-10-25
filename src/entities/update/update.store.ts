import { computed, inject } from "@angular/core";
import { patchState, signalStore, withComputed, withHooks, withMethods, withState } from "@ngrx/signals";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { TuiNotificationService } from '@taiga-ui/core';
import { UpdateInfo, UpdateProgress } from "./update.model";

/** Where the releases are, to update by hand when the app can't (e.g. the Arch package). */
const RELEASES_URL = 'https://github.com/Manolo-dl/Torosu/releases/latest';

interface UpdateState {
    /** The newer version found at startup, if any. */
    available: UpdateInfo | null;
    installing: boolean;
    progress: UpdateProgress | null;
    /** The app could not install it itself: it has to be downloaded. */
    failed: boolean;
}

const initialState: UpdateState = {
    available: null,
    installing: false,
    progress: null,
    failed: false,
};

/** Offers the updates found at startup; nothing is installed until the user asks. */
export const UpdateStore = signalStore(
    { providedIn: 'root' },
    withState(initialState),

    withComputed((store) => ({
        /** Download progress, 0 to 100, or `null` while the size is unknown. */
        percent: computed(() => {
            const p = store.progress();
            return p?.total ? Math.min(100, Math.round(p.downloaded / p.total * 100)) : null;
        }),
    })),

    withMethods((store, alerts = inject(TuiNotificationService)) => ({
        async check() {
            try {
                const available = await invoke<UpdateInfo | null>('check_update');
                patchState(store, { available });
                if (available) {
                    alerts.open(`Torosu ${available.version} is available. Update it from the icon in the header.`, { appearance: 'info' }).subscribe();
                }
            } catch (e) {
                console.warn('failed to check for updates', e);
            }
        },

        /** Downloads and installs the update; the app restarts when it is done. */
        async install() {
            if (!store.available() || store.installing()) return;
            patchState(store, { installing: true, progress: null, failed: false });

            const unlisten = await listen<UpdateProgress>('update:progress', event => {
                patchState(store, { progress: event.payload });
            });
            try {
                await invoke('install_update');
            } catch (e) {
                console.error('install_update failed', e);
                alerts.open('Torosu could not update itself. Download the new version from the releases page.', { appearance: 'negative' }).subscribe();
                patchState(store, { installing: false, progress: null, failed: true });
            } finally {
                unlisten();
            }
        },

        openReleases() {
            openUrl(RELEASES_URL).catch(e => console.error('failed to open the releases page', e));
        },
    })),

    withHooks({
        onInit(store) {
            store.check();
        },
    }),
);
