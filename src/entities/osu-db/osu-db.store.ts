import { computed } from "@angular/core";
import { signalStore, withComputed } from "@ngrx/signals";
import { withLoader } from "@shared/stores";
import { invoke } from "@tauri-apps/api/core";
import { OsuBeatmapSet, OsuDiff, isSubmitted } from "./osu-db.model";

interface OsuDbState {
    beatmapSets: OsuBeatmapSet[];
}

const initialState: OsuDbState = {
    beatmapSets: [],
};

export const OsuDbStore = signalStore(
    { providedIn: 'root' },
    withLoader(initialState, {
        read: () => invoke<OsuBeatmapSet[]>('read_osu_db_full'),
        loaded: beatmapSets => ({ beatmapSets }),
        needsOsuPath: true,
        success: beatmapSets => `Loaded ${beatmapSets.length} beatmap sets from osu!.db`,
        failure: 'Failed to load osu!.db',
    }),

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
)
