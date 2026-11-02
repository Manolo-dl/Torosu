import { patchState, signalStore, withComputed, withMethods } from "@ngrx/signals";
import { OsuCollection } from "./collection.model";
import { computed, inject } from "@angular/core";
import { OsuDbStore, isUncheckedSet, setsOfDiffs } from "@entities/osu-db";
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
        success: collections => `Loaded ${collections.length} collections`,
        failure: 'Failed to load collections',
    }),

    withComputed((store, osuDb = inject(OsuDbStore)) => {
        const collectionsWithBeatmaps = computed(() => {
            // Exported as links, so only what is on the website, with the diffs of the collection.
            const setsByMd5 = osuDb.submittedSetsByMd5();
            return store.collections().map(col => ({ name: col.name, sets: setsOfDiffs(col.md5s, setsByMd5) }));
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
            /** Selected sets osu! never checked online, which may not be on the website. */
            uncheckedSelectedSets: computed(() => selectedSets().filter(isUncheckedSet).length),
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
