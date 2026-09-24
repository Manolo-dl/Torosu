import { patchState, signalStore, withComputed, withMethods, withState } from "@ngrx/signals";
import { TosuData } from "./tosu.model";
import { rxMethod } from "@ngrx/signals/rxjs-interop";
import { from, pipe, switchMap } from "rxjs";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { computed, inject } from "@angular/core";
import { OsuPathStore, ToastStore } from "@shared/stores";

interface TosuStoreState {
    data: TosuData | null;
    connected: boolean;
}

const initialState: TosuStoreState = {
    data: null,
    connected: false,
}

export const TosuStore = signalStore(
    { providedIn: 'root' },
    withState(initialState),

    withComputed((store, osuPath = inject(OsuPathStore)) => ({
        title: computed(() => store.data()?.beatmap.title ?? 'undefined'),
        artist: computed(() => store.data()?.beatmap.artist ?? 'undefined'),
        mapper: computed(() => store.data()?.beatmap.mapper ?? 'undefined'),
        version: computed(() => store.data()?.beatmap.version ?? 'undefined'),
        stars: computed(() => store.data()?.beatmap.stats.stars.total ?? 0),
        backgroundUrl: computed(() => {
            const data = store.data();
            if (!data) return null;
            const path =  convertFileSrc(`${osuPath.path()}/Songs/${data.directPath.beatmapBackground}`);
            return `url('${path}')`;
        }),
    })),

    withMethods((store, toast = inject(ToastStore)) => ({
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
                                });

                                toast.show('success', 'Connected to Tosu successfully.');
                                patchState(store, { connected: true });
                            } catch {
                                toast.show('error', 'Failed to connect to Tosu. Please make sure Tosu is running and try again.');
                                patchState(store, { connected: false });
                            }
                        })()
                    )
                )
            )
        )
    }))
);