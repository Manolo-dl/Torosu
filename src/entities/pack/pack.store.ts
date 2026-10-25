import { patchState, signalStore, withComputed, withHooks, withMethods, withState } from "@ngrx/signals";
import { SelectedDiff } from "./pack.model";
import { invoke } from "@tauri-apps/api/core";
import { computed, inject } from "@angular/core";
import { OsuDbStore } from "@entities/osu-db";
import { OsuCollection } from "@entities/collection";
import { ConfigurationStore } from "@entities/configuration";
import { TuiNotificationService } from '@taiga-ui/core';

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

    withComputed((store, osuDbStore = inject(OsuDbStore)) => ({
        filteredSets: computed(() => {
            const query = store.searchQuery().toLowerCase().trim();
            const sets = osuDbStore.beatmapSets();

            if (!query) return sets;

            return sets.filter(set =>
                set.title.toLowerCase().includes(query) ||
                set.artist.toLowerCase().includes(query) ||
                set.diffs.some(diff => diff.fileName.toLowerCase().includes(query))
            );
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

    withMethods((store, alerts = inject(TuiNotificationService), osuDb = inject(OsuDbStore)) => ({
        toggleDiff(diff: SelectedDiff) {
            const diffs = store.selectedDiffs();
            const index = diffs.findIndex(d => d.md5 === diff.md5);

            if (index === -1) {
                patchState(store, { selectedDiffs: [...diffs, diff] });
            } else {
                const newDiffs = [...diffs];
                newDiffs.splice(index, 1);
                patchState(store, { selectedDiffs: newDiffs });
            }
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

        addFromCollection(collection: OsuCollection) {
            const setsByMd5 = osuDb.beatmapSetsByMd5();
            const diffsByMd5 = osuDb.diffsByMd5();
            const alreadyAdded = store.addedMd5s();

            const newDiffs: SelectedDiff[] = [];
            for (const md5 of collection.md5s) {
                if (alreadyAdded.has(md5)) continue;
                const set = setsByMd5.get(md5);
                const diff = diffsByMd5.get(md5);
                if (!set || !diff) continue;
                newDiffs.push({
                    md5: diff.md5,
                    folderName: set.folderName,
                    fileName: diff.fileName,
                    audio: diff.audio,
                    newDiffName: `[${diff.creator}] ${set.artist} - ${set.title} (${diff.diffName})`,
                });
            }

            if (newDiffs.length > 0) {
                patchState(store, { selectedDiffs: [...store.selectedDiffs(), ...newDiffs] });
            }
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
                alerts.open(message, { appearance: 'positive' }).subscribe();
                patchState(store, initialState);
            } catch (error) {
                patchState(store, { isCreating: false });
                alerts.open('Failed to create pack', { appearance: 'negative' }).subscribe();
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
    })),

    withHooks({
        onInit() {
            // For `packCount`; the backend reads the setting on its own.
            const config = inject(ConfigurationStore);
            if (!config.isLoaded()) config.loadConfig();
        },
    }),
)
