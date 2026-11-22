import { patchState, signalStore, withComputed, withMethods, withState } from "@ngrx/signals";
import { DownloadItem, DownloadStatus } from "./download-queue.model";
import { computed, inject } from "@angular/core";
import { OsuDbStore } from "@entities/osu-db";

interface DownloadQueueState {
    rawQueue: number[];
    statusMap: Record<number, Omit<DownloadItem, 'beatmapSetId'>>;
    isDownloading: boolean;
}

const initialState: DownloadQueueState = {
    rawQueue: [],
    statusMap: {},
    isDownloading: false
};

/** The beatmap sets to download, and how each one is going. */
export const DownloadQueueStore = signalStore(
    { providedIn: 'root' },
    withState(initialState),

    withComputed((store, osuDb = inject(OsuDbStore)) => ({
        queue: computed(() => {
            const localIds = osuDb.localSubmittedSetIds();
            return store.rawQueue()
                .filter(id => !localIds.has(id))
                .map((id): DownloadItem => ({
                    beatmapSetId: id,
                    ...(store.statusMap()[id] ?? { status: 'pending' as DownloadStatus, progress: 0 })
                }));
        }),

        skippedCount: computed(() =>
            store.rawQueue().filter(id => osuDb.localSubmittedSetIds().has(id)).length
        ),
    })),

    withMethods((store) => ({

        addToQueue(ids: number[]) {
            const existing = new Set(store.rawQueue());
            const statusMap = { ...store.statusMap() };

            const newIds: number[] = [];
            for (const id of ids) {
                if (!existing.has(id)) {
                    newIds.push(id);
                } else {
                    const s = statusMap[id]?.status;
                    if (s === 'done' || s === 'failed') {
                        delete statusMap[id];
                    }
                }
            }

            patchState(store, {
                rawQueue: [...store.rawQueue(), ...newIds],
                statusMap,
            });
        },

        /** Its name is kept from the reports that brought it. */
        updateStatus(beatmapSetId: number, status: DownloadStatus, progress = 0, name?: string) {
            const previous = store.statusMap()[beatmapSetId];
            patchState(store, {
                statusMap: {
                    ...store.statusMap(),
                    [beatmapSetId]: { status, progress, name: name ?? previous?.name }
                }
            });
        },

        clearQueue() {
            patchState(store, { rawQueue: [], statusMap: {} });
        },

        setDownloading(isDownloading: boolean) {
            patchState(store, { isDownloading });
        },

        removeItem(beatmapSetId: number) {
            const { [beatmapSetId]: _, ...rest } = store.statusMap();
            patchState(store, {
                rawQueue: store.rawQueue().filter(id => id !== beatmapSetId),
                statusMap: rest,
            });
        }
    })),
);