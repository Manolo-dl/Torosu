import { patchState, signalStore, withComputed, withMethods, withState } from "@ngrx/signals";
import { OsuCollection } from "./collection.model";
import { computed, inject } from "@angular/core";
import { OsuDbStore } from "@entities/osu-db";
import { OsuBeatmapSet } from "@entities/osu-db/osu-db.model";
import { OsuPathStore, ToastStore } from "@shared/stores"; 
import { invoke } from "@tauri-apps/api/core";


interface CollectionState {
    collections: OsuCollection[];
    selectedCollections: string[];
    isLoading: boolean;
    isLoaded: boolean;
}

const initialState: CollectionState = {
    collections: [],
    selectedCollections: [],
    isLoading: false,
    isLoaded: false
};

export const CollectionStore = signalStore(
    { providedIn: 'root' },
    withState(initialState),

    withComputed((store, osuDb = inject(OsuDbStore)) => {
        const collectionsWithBeatmaps = computed(() => {
            const setsByMd5 = osuDb.beatmapSetsByMd5();
            return store.collections().map(col => {
                const seenIds = new Set<number>();
                const sets = col.md5s
                    .map(md5 => setsByMd5.get(md5))
                    .filter((set): set is OsuBeatmapSet => set !== undefined)
                    .filter(set => {
                        if (seenIds.has(set.beatmapsetId)) return false;
                        seenIds.add(set.beatmapsetId);
                        return true;
                    });
                return { name: col.name, sets };
            });
        });

        const selectedSets = computed(() => {
            const selected = store.selectedCollections();
            const seen = new Set<number>();
            const sets: OsuBeatmapSet[] = [];
            for (const col of collectionsWithBeatmaps()) {
                if (!selected.includes(col.name)) continue;
                for (const set of col.sets) {
                    if (seen.has(set.beatmapsetId)) continue;
                    seen.add(set.beatmapsetId);
                    sets.push(set);
                }
            }
            return sets;
        });

        return {
            collectionsWithBeatmaps,
            selectedSets,
            selectedCount: computed(() => store.selectedCollections().length),
            totalSelectedSets: computed(() => selectedSets().length),
            allSelected: computed(() => {
                const cols = collectionsWithBeatmaps();
                return cols.length > 0 && cols.length === store.selectedCollections().length;
            }),
        };
    }),

    withMethods((store, osuPath = inject(OsuPathStore), toast = inject(ToastStore)) => ({

        async load() {
            const path = osuPath.path();
            if (!path) {
                toast.show('error', 'Osu! path not set');
                throw new Error("Osu! path not set");
            }

            patchState(store, { isLoading: true });

            try {
                const collections = await invoke<OsuCollection[]>("read_osu_collections");
                toast.show('success', `Loaded ${collections.length} collections`);
                patchState(store, { collections, isLoaded: true, isLoading: false });
            } catch (error) {
                patchState(store, { isLoading: false });
                toast.show('error', `Failed to load collections`);
                throw error;
            }
        },

        setCollections(collections: OsuCollection[]) {
            patchState(store, { collections, isLoaded: true, isLoading: false });
        },

        setLoading(isLoading: boolean) {
            patchState(store, { isLoading });
        },

        toggleSelection(name: string) {
            const selected = store.selectedCollections();
            patchState(store, {
                selectedCollections: selected.includes(name)
                    ? selected.filter(c => c !== name)
                    : [...selected, name]
            });
        },

        selectAll() {
            patchState(store, {
                selectedCollections: store.collections().map(c => c.name)
            });
        },

        clearSelection() {
            patchState(store, { selectedCollections: [] });
        },

        reset() {
            patchState(store, initialState);
        }
    })),
);
