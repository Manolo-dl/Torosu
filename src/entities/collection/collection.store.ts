import { patchState, signalStore, withMethods } from "@ngrx/signals";
import { OsuCollection } from "./collection.model";
import { withLoader } from "@shared/stores";
import { invoke } from "@tauri-apps/api/core";


interface CollectionState {
    collections: OsuCollection[];
}

const initialState: CollectionState = {
    collections: [],
};

export const CollectionStore = signalStore(
    { providedIn: 'root' },
    withLoader(initialState, {
        read: () => invoke<OsuCollection[]>('read_osu_collections'),
        loaded: collections => ({ collections }),
        success: collections => ({ key: 'loading.collectionsDone', params: { count: collections.length } }),
        failure: 'loading.collectionsFailed',
    }),

    withMethods((store) => ({
        /** collection.db as it is after Torosu wrote it (an import), without telling it was loaded. */
        setCollections(collections: OsuCollection[]) {
            patchState(store, { collections });
        },
    })),
);
