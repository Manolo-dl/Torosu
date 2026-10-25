import { patchState, signalStore, withComputed, withMethods, withState } from "@ngrx/signals";
import { OsuCollection } from "./collection.model";
import { computed, inject } from "@angular/core";
import { OsuDbStore } from "@entities/osu-db";
import { OsuBeatmapSet } from "@entities/osu-db/osu-db.model";
import { OsuPathStore } from "@shared/stores"; 
import { TuiNotificationService } from '@taiga-ui/core';
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
            // Exported as links, so only what is on the website, with the diffs of the collection.
            const setsByMd5 = osuDb.submittedSetsByMd5();
            return store.collections().map(col => {
                const md5s = new Set(col.md5s);
                const seenFolders = new Set<string>();
                const sets: OsuBeatmapSet[] = [];
                for (const md5 of md5s) {
                    const set = setsByMd5.get(md5);
                    if (!set || seenFolders.has(set.folderName)) continue;
                    seenFolders.add(set.folderName);
                    sets.push({ ...set, diffs: set.diffs.filter(d => md5s.has(d.md5)) });
                }
                return { name: col.name, sets };
            });
        });

        const selectedSets = computed(() => {
            const selected = store.selectedCollections();
            const seen = new Set<string>();
            const sets: OsuBeatmapSet[] = [];
            for (const col of collectionsWithBeatmaps()) {
                if (!selected.includes(col.name)) continue;
                for (const set of col.sets) {
                    if (seen.has(set.folderName)) continue;
                    seen.add(set.folderName);
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

    withMethods((store, osuPath = inject(OsuPathStore), alerts = inject(TuiNotificationService)) => ({

        async load() {
            const path = osuPath.path();
            if (!path) {
                alerts.open('Osu! path not set', { appearance: 'negative' }).subscribe();
                throw new Error("Osu! path not set");
            }

            patchState(store, { isLoading: true });

            try {
                const collections = await invoke<OsuCollection[]>("read_osu_collections");
                alerts.open(`Loaded ${collections.length} collections`, { appearance: 'positive' }).subscribe();
                patchState(store, { collections, isLoaded: true, isLoading: false });
            } catch (error) {
                patchState(store, { isLoading: false });
                alerts.open(`Failed to load collections`, { appearance: 'negative' }).subscribe();
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
