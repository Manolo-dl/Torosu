import { patchState, signalStore, withComputed, withMethods } from "@ngrx/signals";
import { OsuCollection } from "./collection.model";
import { computed, inject } from "@angular/core";
import { OsuDbStore } from "@entities/osu-db";
import { OsuBeatmapSet } from "@entities/osu-db/osu-db.model";
import { withLoader } from "@shared/stores";
import { invoke } from "@tauri-apps/api/core";


interface CollectionState {
    collections: OsuCollection[];
    selectedCollections: string[];
}

const initialState: CollectionState = {
    collections: [],
    selectedCollections: [],
};

export const CollectionStore = signalStore(
    { providedIn: 'root' },
    withLoader(initialState, {
        read: () => invoke<OsuCollection[]>('read_osu_collections'),
        loaded: collections => ({ collections }),
        needsOsuPath: true,
        success: collections => `Loaded ${collections.length} collections`,
        failure: 'Failed to load collections',
    }),

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

    withMethods((store) => ({
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
    })),
);
