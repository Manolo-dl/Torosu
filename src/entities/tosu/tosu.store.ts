import { patchState, signalStore, withComputed, withMethods, withState } from "@ngrx/signals";
import { TosuData } from "./tosu.model";
import { rxMethod } from "@ngrx/signals/rxjs-interop";
import { from, pipe, switchMap } from "rxjs";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { computed, inject } from "@angular/core";
import { TuiNotificationService } from '@taiga-ui/core';

interface TosuStoreState {
    data: TosuData | null;
    connected: boolean;
    /** `blob:` URL of the current beatmap background, see `loadBackground`. */
    backgroundUrl: string | null;
}

const initialState: TosuStoreState = {
    data: null,
    connected: false,
    backgroundUrl: null,
}

const IMAGE_TYPES: Record<string, string> = {
    jpg: 'image/jpeg',
    jpeg: 'image/jpeg',
    png: 'image/png',
    gif: 'image/gif',
    bmp: 'image/bmp',
    webp: 'image/webp',
};

export const TosuStore = signalStore(
    { providedIn: 'root' },
    withState(initialState),

    withComputed((store) => ({
        title: computed(() => store.data()?.beatmap.title ?? 'undefined'),
        artist: computed(() => store.data()?.beatmap.artist ?? 'undefined'),
        mapper: computed(() => store.data()?.beatmap.mapper ?? 'undefined'),
        version: computed(() => store.data()?.beatmap.version ?? 'undefined'),
        stars: computed(() => store.data()?.beatmap.stats.stars.total ?? 0),
    })),

    withMethods((store, alerts = inject(TuiNotificationService)) => {
        // Path of the background being shown (or loading), to skip reloading it
        // on every tosu update and to drop responses that arrive late.
        let backgroundPath: string | null = null;

        // The background is read by the backend and shown through a blob URL:
        // WebKitGTK blocks `asset:` URLs from the dev server origin.
        async function loadBackground(path: string | null) {
            if (path === backgroundPath) return;
            backgroundPath = path;

            let url: string | null = null;
            if (path) {
                try {
                    const bytes = await invoke<ArrayBuffer>('read_songs_file', { path });
                    const ext = path.split('.').pop()?.toLowerCase() ?? '';
                    url = URL.createObjectURL(new Blob([bytes], { type: IMAGE_TYPES[ext] ?? '' }));
                } catch (e) {
                    console.warn('failed to load beatmap background', e);
                }
            }

            if (path !== backgroundPath) {
                if (url) URL.revokeObjectURL(url);
                return;
            }

            const previous = store.backgroundUrl();
            patchState(store, { backgroundUrl: url });
            if (previous) URL.revokeObjectURL(previous);
        }

        return {
            connect: rxMethod<void>(
                pipe(
                    switchMap(() =>
                        from(
                            (async () => {
                                if (store.connected()) return;
                            
                                try {
                                    await invoke('connect_tosu');
                                    await listen<TosuData>('tosu-data', (event) => {
                                        patchState(store, { data: event.payload });
                                        loadBackground(event.payload.directPath.beatmapBackground || null);
                                    });

                                    alerts.open('Connected to Tosu successfully.', { appearance: 'positive' }).subscribe();
                                    patchState(store, { connected: true });
                                } catch {
                                    alerts.open('Failed to connect to Tosu. Please make sure Tosu is running and try again.', { appearance: 'negative' }).subscribe();
                                    patchState(store, { connected: false });
                                }
                            })()
                        )
                    )
                )
            )
        };
    })
);