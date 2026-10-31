import { patchState, signalStore, withComputed, withMethods, withState } from "@ngrx/signals";
import { OsuReaderSnapshot, OsuReaderStatus, SelectedBeatmap } from "./osu-reader.model";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { computed, inject } from "@angular/core";
import { NotifyService } from '@shared/services';
import { save } from '@tauri-apps/plugin-dialog';

interface OsuReaderStoreState {
    data: SelectedBeatmap | null;
    status: OsuReaderStatus;
    /** Whether the backend reader was started; it keeps looking for osu! on its own. */
    started: boolean;
    /** `blob:` URL of the current beatmap background, see `loadBackground`. */
    backgroundUrl: string | null;
    exporting: boolean;
}

const initialState: OsuReaderStoreState = {
    data: null,
    status: { state: 'waiting' },
    started: false,
    backgroundUrl: null,
    exporting: false,
}

const IMAGE_TYPES: Record<string, string> = {
    jpg: 'image/jpeg',
    jpeg: 'image/jpeg',
    png: 'image/png',
    gif: 'image/gif',
    bmp: 'image/bmp',
    webp: 'image/webp',
};

export const OsuReaderStore = signalStore(
    { providedIn: 'root' },
    withState(initialState),

    withComputed((store) => ({
        connected: computed(() => store.status().state === 'connected'),
        title: computed(() => store.data()?.beatmap.title ?? 'undefined'),
        artist: computed(() => store.data()?.beatmap.artist ?? 'undefined'),
        mapper: computed(() => store.data()?.beatmap.creator ?? 'undefined'),
        version: computed(() => store.data()?.beatmap.difficulty ?? 'undefined'),
        stars: computed(() => store.data()?.attributes?.stars ?? 0),
        /** Mode the map was made for; `null` if its `.osu` file could not be read. */
        originalMode: computed(() => store.data()?.attributes?.originalMode ?? null),
    })),

    withMethods((store, notify = inject(NotifyService)) => {
        // Path of the background being shown (or loading), to skip reloading it
        // and to drop responses that arrive late.
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

        function setBeatmap(data: SelectedBeatmap | null) {
            patchState(store, { data });
            loadBackground(data?.paths?.background ?? null);
        }

        function setStatus(status: OsuReaderStatus) {
            const previous = store.status();
            patchState(store, { status });
            switch (status.state) {
                case 'waiting':
                    if (previous.state === 'connected') {
                        notify.info('osu! was closed, waiting for it to start again.');
                    }
                    break;
                case 'connected':
                    notify.success('Connected to osu!');
                    break;
                case 'error':
                    notify.error(`Can't read osu!: ${status.message}`);
                    break;
            }
        }

        return {
            /** Saves the set folder of the selected beatmap as an .osz, where the user picks. */
            async exportOsz() {
                const data = store.data();
                if (!data?.paths || store.exporting()) return;

                const { setId, artist, title } = data.beatmap;
                // Same name osu! gives exported sets.
                const name = `${setId > 0 ? `${setId} ` : ''}${artist} - ${title}.osz`.replace(/[\\/:*?"<>|]/g, '_');
                const path = await save({
                    filters: [{ name: 'osu! beatmap set', extensions: ['osz'] }],
                    defaultPath: name,
                });
                if (!path) return;

                patchState(store, { exporting: true });
                try {
                    await invoke('export_selected_osz', { path });
                    notify.success(`Exported ${name}`);
                } catch (e) {
                    console.error('export_selected_osz failed', e);
                    notify.error(`Failed to export the beatmap: ${e}`);
                } finally {
                    patchState(store, { exporting: false });
                }
            },

            async connect() {
                if (store.started()) return;
                patchState(store, { started: true });

                try {
                    // Listen first: the snapshot covers what happened before.
                    await listen<OsuReaderStatus>('osu-reader-status', event => setStatus(event.payload));
                    await listen<SelectedBeatmap>('osu-beatmap', event => setBeatmap(event.payload));

                    // An event that arrived meanwhile is newer than the snapshot.
                    const snapshot = await invoke<OsuReaderSnapshot>('connect_osu_reader');
                    if (snapshot.beatmap && !store.data()) setBeatmap(snapshot.beatmap);
                    if (store.status().state === 'waiting') {
                        if (snapshot.status.state === 'waiting') {
                            notify.info('Looking for osu!...');
                        } else {
                            setStatus(snapshot.status);
                        }
                    }
                } catch (e) {
                    console.error('connect_osu_reader failed', e);
                    notify.error('Failed to start the osu! reader.');
                    patchState(store, { started: false });
                }
            },
        };
    })
);
