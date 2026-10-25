import { computed, inject } from "@angular/core";
import { patchState, signalStore, withComputed, withMethods, withState } from "@ngrx/signals";
import { OsuPathStore } from "@shared/stores"; 
import { TuiNotificationService } from '@taiga-ui/core';
import { invoke } from "@tauri-apps/api/core";
import { OsuBeatmapSet, OsuDiff, isSubmitted } from "./osu-db.model";

interface OsuDbState {
    beatmapSets: OsuBeatmapSet[];
    isLoading: boolean;
    isLoaded: boolean;
}

const initialState: OsuDbState = {
    beatmapSets: [],
    isLoading: false,
    isLoaded: false,
};

export const OsuDbStore = signalStore(
    { providedIn: 'root' },
    withState(initialState),

    withComputed((store) => ({
        diffsByMd5: computed(() => {
            const map = new Map<string, OsuDiff>();
            for (const set of store.beatmapSets()) {
                for (const diff of set.diffs) {
                    map.set(diff.md5, diff);
                }
            }
            return map;
        }),

        beatmapSetsByMd5: computed(() => {
            const map = new Map<string, OsuBeatmapSet>();
            for (const set of store.beatmapSets()) {
                for (const diff of set.diffs) {
                    map.set(diff.md5, set);
                }
            }
            return map;
        }),

        totalSets: computed(() => store.beatmapSets().length),

        totalDiffs: computed(() =>
            store.beatmapSets().reduce((sum, s) => sum + s.diffs.length, 0)
        ),

        countByStatus: computed(() => {
            const counts: Record<string, number> = {};
            for (const s of store.beatmapSets()) {
                for (const d of s.diffs) {
                    counts[d.status] = (counts[d.status] || 0) + 1;
                }
            }
            return counts;
        }),

        countByMode: computed(() => {
            const counts: Record<number, number> = {};
            for (const s of store.beatmapSets()) {
                for (const d of s.diffs) {
                    counts[d.mode] = (counts[d.mode] ?? 0) + 1;
                }
            }
            return counts;
        }),

        /**
         * Only what is on the osu! website, for the tools that link to or download
         * sets: sets without an id are left out, and so are the diffs of the other
         * sets that were never uploaded (e.g. rates). Create Packs uses `beatmapSets`.
         */
        submittedBeatmapSets: computed(() =>
            store.beatmapSets()
                .filter(set => set.beatmapsetId > 0)
                .map(set => ({ ...set, diffs: set.diffs.filter(isSubmitted) }))
                .filter(set => set.diffs.length > 0)
        ),
    })),

    withComputed((store) => ({
        submittedSetsByMd5: computed(() => {
            const map = new Map<string, OsuBeatmapSet>();
            for (const set of store.submittedBeatmapSets()) {
                for (const diff of set.diffs) {
                    map.set(diff.md5, set);
                }
            }
            return map;
        }),

        /** An id can be in several folders (e.g. the set downloaded twice). */
        submittedSetsById: computed(() => {
            const map = new Map<number, OsuBeatmapSet[]>();
            for (const set of store.submittedBeatmapSets()) {
                const sets = map.get(set.beatmapsetId);
                if (sets) sets.push(set);
                else map.set(set.beatmapsetId, [set]);
            }
            return map;
        }),

        localSubmittedSetIds: computed(() => new Set(store.submittedBeatmapSets().map(s => s.beatmapsetId))),
    })),

    withMethods((store, osuPath = inject(OsuPathStore), alerts = inject(TuiNotificationService)) => ({
        setBeatmapSets(beatmapSets: OsuBeatmapSet[]) {
            patchState(store, { beatmapSets, isLoaded: true, isLoading: false });
        },

        setLoading(isLoading: boolean) {
            patchState(store, { isLoading });
        },

        reset() {
            patchState(store, initialState);
        },

        async load() {
            const path = osuPath.path();
            if (!path) {
                alerts.open('Osu! path not set', { appearance: 'negative' }).subscribe();
                throw new Error("Osu! path not set");
            } 

            patchState(store, { isLoading: true });

            try {
                const beatmapSets = await invoke<OsuBeatmapSet[]>('read_osu_db_full');
                alerts.open(`Loaded ${beatmapSets.length} beatmap sets from osu!.db`, { appearance: 'positive' }).subscribe();
                patchState(store, { beatmapSets, isLoaded: true, isLoading: false });
            } catch (error) {
                patchState(store, { isLoading: false });
                alerts.open(`Failed to load osu!.db`, { appearance: 'negative' }).subscribe();
                throw error;
            }
        }
    })),
)