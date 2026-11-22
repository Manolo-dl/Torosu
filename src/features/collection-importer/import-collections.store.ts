import { errorText } from '@shared/i18n';
import { TranslocoService } from '@jsverse/transloco';
import { computed, inject } from "@angular/core";
import { CollectionStore, OsuCollection } from "@entities/collection";
import { OsuDbStore, setsOfDiffs } from "@entities/osu-db";
import { patchState, signalStore, withComputed, withMethods, withState } from "@ngrx/signals";
import { NotifyService } from '@shared/services';
import { invoke } from "@tauri-apps/api/core";
import { ParsedImport } from "./import-collection-parser.service";

/** `ImportResult` in collections/model.rs. */
export interface ImportResult {
    maps: number;
    created: number;
    merged: number;
}

interface ImportCollectionState {
    parsedImports: ParsedImport[];
    /** Collections of the file left out of the import, by name. */
    excluded: string[];
    isImporting: boolean;
    /** What the last import changed; `null` until there is one. */
    result: ImportResult | null;
    /** Whether osu! is running (it must not be, to import); `null` until known. */
    osuRunning: boolean | null;
}

const initialState: ImportCollectionState = {
    parsedImports: [],
    excluded: [],
    isImporting: false,
    result: null,
    osuRunning: null,
};

/** A collection of the file as it would be imported. */
export interface ImportGroup {
    name: string;
    /** There is one of this name already: the maps go into it. */
    isExisting: boolean;
    /** The maps to add: not in the collection of this name yet. */
    md5s: string[];
    /** Maps of the file that are in that collection already. */
    already: number;
    /** Of `md5s`, those not in the library: osu! lists them as missing until downloaded. */
    notInstalled: number;
    /** Links to sets not in the library: with no diffs to go by, they can't be added. */
    unresolvedSets: number;
    selected: boolean;
}

export const ImportCollectionStore = signalStore(
    { providedIn: 'root' },
    withState(initialState),

    withComputed((store, collectionStore = inject(CollectionStore), osuDb = inject(OsuDbStore)) => {
        const existingMd5sByName = computed(() =>
            new Map(collectionStore.collections().map(c => [c.name, new Set(c.md5s)]))
        );

        const groups = computed<ImportGroup[]>(() => {
            const bySetId = osuDb.submittedSetsById();
            const setsByMd5 = osuDb.beatmapSetsByMd5();
            const excluded = new Set(store.excluded());

            return store.parsedImports().map(imp => {
                // A link stands for every diff of its set in the library.
                const installedSets = imp.beatmapsetIds.map(id => bySetId.get(id) ?? []);
                const fromLinks = installedSets.flatMap(sets => sets.flatMap(set => set.diffs.map(d => d.md5)));
                const all = [...new Set([...imp.md5s, ...fromLinks])];
                const existing = existingMd5sByName().get(imp.name);
                const md5s = existing ? all.filter(md5 => !existing.has(md5)) : all;
                return {
                    name: imp.name,
                    isExisting: existing !== undefined,
                    md5s,
                    already: all.length - md5s.length,
                    notInstalled: md5s.filter(md5 => !setsByMd5.has(md5)).length,
                    unresolvedSets: installedSets.filter(sets => sets.length === 0).length,
                    selected: md5s.length > 0 && !excluded.has(imp.name),
                };
            });
        });

        const selectedGroups = computed(() => groups().filter(group => group.selected));

        return {
            groups,
            selectedGroups,
            /** Maps the file brings, in all its collections. */
            fileMaps: computed(() => groups().reduce((sum, g) => sum + g.md5s.length + g.already, 0)),
            totalMaps: computed(() => selectedGroups().reduce((sum, g) => sum + g.md5s.length, 0)),
            /** Collections with something to add, picked or not. */
            importable: computed(() => groups().filter(group => group.md5s.length > 0).length),
            canImport: computed(() =>
                selectedGroups().length > 0 && store.osuRunning() === false && !store.isImporting()
            ),
        };
    }),

    withMethods((store, notify = inject(NotifyService), transloco = inject(TranslocoService), collectionStore = inject(CollectionStore), osuDb = inject(OsuDbStore)) => {
      async function checkOsu() {
          const osuRunning = await invoke<boolean>('is_osu_running').catch(() => null);
          if (osuRunning !== null) patchState(store, { osuRunning });
      }

      return {
        checkOsu,

        setParsed(parsedImports: ParsedImport[]) {
            patchState(store, { parsedImports, excluded: [], result: null });
        },

        toggle(name: string) {
            const excluded = store.excluded();
            patchState(store, {
                excluded: excluded.includes(name) ? excluded.filter(n => n !== name) : [...excluded, name],
            });
        },

        /** Every collection with something to add picked, or none if they all were. */
        toggleAll() {
            const importable = store.groups().filter(group => group.md5s.length > 0);
            const allPicked = importable.every(group => group.selected);
            patchState(store, { excluded: allPicked ? importable.map(group => group.name) : [] });
        },

        /** The maps of a collection to add, as sets of the library. */
        setsOf(group: ImportGroup) {
            return setsOfDiffs(group.md5s, osuDb.beatmapSetsByMd5());
        },

        clear() {
            patchState(store, { parsedImports: [], excluded: [], result: null });
        },

        /** What the last import changed is told once: not when the page opens again. */
        forgetResult() {
            patchState(store, { result: null });
        },

        async importCollections() {
            const collections = store.selectedGroups().map(({ name, md5s }) => ({ name, md5s }));
            if (collections.length === 0) return;

            patchState(store, { isImporting: true });
            try {
                const result = await invoke<ImportResult>('import_collections', { collections });
                patchState(store, { parsedImports: [], excluded: [], isImporting: false, result });
                // collection.db as it is now, so the collections are known as they are.
                const written = await invoke<OsuCollection[]>('read_osu_collections').catch(() => null);
                if (written) collectionStore.setCollections(written);
            } catch (error) {
                notify.error(transloco.translate('collections.import.failed', { error: errorText(transloco, error) }));
                patchState(store, { isImporting: false });
                // osu! opened meanwhile, maybe: the next check says so at once.
                await checkOsu();
            }
        },
      };
    }),
)
