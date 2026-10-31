import { computed, inject } from "@angular/core";
import { CollectionStore } from "@entities/collection";
import { OsuDbStore } from "@entities/osu-db";
import { patchState, signalStore, withComputed, withMethods, withState } from "@ngrx/signals";
import { NotifyService } from '@shared/services';
import { invoke } from "@tauri-apps/api/core";
import { ParsedImport } from "./import-collection-parser.service";


interface ImportCollectionState {
    parsedImports: ParsedImport[];
    isImporting: boolean;
}

const initialState: ImportCollectionState = {
    parsedImports: [],
    isImporting: false,
};

export const ImportCollectionStore = signalStore(
    { providedIn: 'root' },
    withState(initialState),

    withComputed((store, collectionStore = inject(CollectionStore), osuDB = inject(OsuDbStore)) => {
        const existingMd5sByName = computed(() =>
            new Map(collectionStore.collections().map(c => [c.name, new Set(c.md5s)]))
        );

        // Resolve beatmapsetIds from URL-format imports to all their diffs' md5s
        const resolved = computed(() => {
            const bySetId = osuDB.submittedSetsById();
            return store.parsedImports().map(imp => {
                const urlMd5s = imp.beatmapsetIds.flatMap(id =>
                    (bySetId.get(id) ?? []).flatMap(set => set.diffs.map(d => d.md5))
                );
                return { name: imp.name, md5s: [...imp.md5s, ...urlMd5s] };
            });
        });

        const collectionsToImport = computed(() =>
            resolved()
                .map(col => {
                    const existing = existingMd5sByName().get(col.name);
                    const md5s = existing
                        ? col.md5s.filter(md5 => !existing.has(md5))
                        : col.md5s;
                    return { ...col, md5s };
                })
                .filter(col => col.md5s.length > 0)
        );

        return {
            collectionsToImport,

            totalMaps: computed(() =>
                collectionsToImport().reduce((sum, c) => sum + c.md5s.length, 0)
            ),

            previewGroups: computed(() => {
                const beatmapSetsByMd5 = osuDB.beatmapSetsByMd5();

                return collectionsToImport().map(col => {
                    const md5s = new Set(col.md5s);
                    const seen = new Set<string>();
                    const sets = [];

                    for (const md5 of md5s) {
                        const set = beatmapSetsByMd5.get(md5);
                        if (!set || seen.has(set.folderName)) continue;
                        seen.add(set.folderName);
                        sets.push({ ...set, diffs: set.diffs.filter(d => md5s.has(d.md5)) });
                    }

                    const isExisting = existingMd5sByName().has(col.name);
                    return { name: col.name, sets, isExisting };
                });
            }),
        };
    }),

    withMethods((store, notify = inject(NotifyService)) => ({
        setParsed(parsedImports: ParsedImport[]) {
            patchState(store, { parsedImports });
        },

        clear() {
            patchState(store, initialState);
        },

        async importCollections() {
            const collections = store.collectionsToImport();
            if (collections.length === 0) return;

            patchState(store, { isImporting: true });

            try {
                await invoke('import_collections', { collections });
                notify.success('Collections imported successfully');
                patchState(store, initialState);
            } catch (error) {
                notify.error('Failed to import collections');
                patchState(store, { isImporting: false });
                throw error;
            }
        }
    })),
)
