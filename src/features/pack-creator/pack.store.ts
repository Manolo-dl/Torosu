import { patchState, signalStore, withComputed, withHooks, withMethods, withState } from "@ngrx/signals";
import { OsuBeatmapSet, OsuDiff } from "@entities/osu-db";
import { SelectedDiff, nameSource, selectedDiff } from "./pack.model";
import { invoke } from "@tauri-apps/api/core";
import { computed, inject } from "@angular/core";
import { OsuDbStore } from "@entities/osu-db";
import { OsuCollection } from "@entities/collection";
import { ConfigurationStore } from "@entities/configuration";
import { NotifyService } from '@shared/services';

interface PackStoreState {
    selectedDiffs: SelectedDiff[];
    title: string;
    finalCreator: string;
    isCreating: boolean;
    searchQuery: string;
    titleValid: boolean;
}

const initialState: PackStoreState = {
    selectedDiffs: [],
    title: '',
    finalCreator: '',
    isCreating: false,
    searchQuery: '',
    titleValid: false,
}
export const PackStore = signalStore(
    { providedIn: 'root' },
    withState(initialState),

    withComputed((_, osuDbStore = inject(OsuDbStore)) => ({
        /**
         * What the search looks in, lowercased once per library instead of at
         * every search: title, artist and diff file names, one per line (a
         * query is a single line, so it can't match across two of them).
         */
        _searchIndex: computed(() =>
            osuDbStore.beatmapSets().map(set => ({
                set,
                text: [set.title, set.artist, ...set.diffs.map(diff => diff.fileName)].join('\n').toLowerCase(),
            }))
        ),
    })),

    withComputed((store, osuDbStore = inject(OsuDbStore)) => ({
        filteredSets: computed(() => {
            const query = store.searchQuery().toLowerCase().trim();
            if (!query) return osuDbStore.beatmapSets();

            return store._searchIndex().filter(entry => entry.text.includes(query)).map(entry => entry.set);
        }),

        addedMd5s: computed(() =>
            new Set(store.selectedDiffs().map(diff => diff.md5))
        ),

        duplicateNames: computed(() => {
            const diffs = store.selectedDiffs();
            const nameCount = new Map<string, number>();
            diffs.forEach(d => {
                if (d.newDiffName.trim() === '') return;
                nameCount.set(d.newDiffName, (nameCount.get(d.newDiffName) ?? 0) + 1);
            });
            return new Set(
                diffs.filter(d => (nameCount.get(d.newDiffName) ?? 0) > 1).map(d => d.md5)
            );
        }),

        hasEmptyNames: computed(() =>
            store.selectedDiffs().some(d => d.newDiffName.trim() === '')
        ),

        /** Original sets the diffs come from; each brings its own audio and background. */
        setCount: computed(() => new Set(store.selectedDiffs().map(d => d.folderName)).size),
    })),

    withComputed((store, config = inject(ConfigurationStore)) => ({
        /** Packs the selection is split into, as the backend does with `max_sets_per_pack`. */
        packCount: computed(() => {
            const sets = store.setCount();
            const max = config.config()?.packs.max_sets_per_pack ?? 0;
            return max > 0 && sets > max ? Math.ceil(sets / max) : 1;
        }),
    })),

    withMethods((store, notify = inject(NotifyService), osuDb = inject(OsuDbStore), config = inject(ConfigurationStore)) => {
        /** The diffs picked for the pack, named with the name format in the settings. */
        async function named(picked: { set: OsuBeatmapSet; diff: OsuDiff }[]): Promise<SelectedDiff[]> {
            const names = await invoke<string[]>('pack_diff_names', {
                format: config.config()?.packs.name_format ?? '',
                diffs: picked.map(({ set, diff }) => nameSource(set, diff)),
            });
            return picked.map(({ set, diff }, i) => selectedDiff(set, diff, names[i]));
        }

        /** Adds the ones not added yet: naming them takes a moment, so they may be by then. */
        function addNew(diffs: SelectedDiff[]) {
            const added = new Set(store.selectedDiffs().map(d => d.md5));
            const fresh = diffs.filter(d => !added.has(d.md5));
            if (fresh.length > 0) patchState(store, { selectedDiffs: [...store.selectedDiffs(), ...fresh] });
        }

        return {
            async addDiff(set: OsuBeatmapSet, diff: OsuDiff) {
                addNew(await named([{ set, diff }]));
            },

            removeDiff(md5: string) {
                patchState(store, { selectedDiffs: store.selectedDiffs().filter(d => d.md5 !== md5) });
            },

            updateDiffName(diff: SelectedDiff, newDiffName: string) {
                const diffs = store.selectedDiffs();
                const index = diffs.findIndex(d => d.md5 === diff.md5);

                if (index >= 0 && diffs[index].newDiffName !== newDiffName) {
                    const newDiffs = [...diffs];
                    newDiffs[index] = { ...newDiffs[index], newDiffName };
                    patchState(store, { selectedDiffs: newDiffs });
                }
            },

            async addFromCollection(collection: OsuCollection) {
                const setsByMd5 = osuDb.beatmapSetsByMd5();
                const diffsByMd5 = osuDb.diffsByMd5();
                const alreadyAdded = store.addedMd5s();

                const picked: { set: OsuBeatmapSet; diff: OsuDiff }[] = [];
                for (const md5 of collection.md5s) {
                    if (alreadyAdded.has(md5)) continue;
                    const set = setsByMd5.get(md5);
                    const diff = diffsByMd5.get(md5);
                    if (set && diff) picked.push({ set, diff });
                }

                if (picked.length > 0) addNew(await named(picked));
            },

            setTitle(title: string) {
                patchState(store, { title });
            },

            clear() {
                patchState(store, initialState);
            },

            async createPack() {
                try {
                    patchState(store, { isCreating: true });
                    await invoke('create_pack', {
                        request: {
                            title: store.title(),
                            finalCreator: store.finalCreator(),
                            diffs: store.selectedDiffs()
                        }
                    });
                    patchState(store, { isCreating: false });
                    const message = store.packCount() > 1
                        ? `Pack ${store.title()} created successfully in ${store.packCount()} parts!`
                        : `Pack ${store.title()} created successfully!`;
                    notify.success(message);
                    patchState(store, initialState);
                } catch (error) {
                    patchState(store, { isCreating: false });
                    notify.error('Failed to create pack');
                    throw error;
                }
            },

            setSearchQuery(query: string) {
                patchState(store, { searchQuery: query });
            },

            setFinalCreator(finalCreator: string) {
                patchState(store, { finalCreator });
            },

            setTitleValid(titleValid: boolean) {
                patchState(store, { titleValid });
            }
        };
    }),

    withHooks({
        onInit() {
            // For `packCount`; the backend reads the setting on its own.
            const config = inject(ConfigurationStore);
            config.ensureLoaded();
        },
    }),
)
