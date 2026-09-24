import { computed, inject } from "@angular/core";
import { OsuDbStore } from "@entities/osu-db";
import { patchState, signalStore, withComputed, withMethods, withState } from "@ngrx/signals";

export interface ModeFilters {
    status: string | null;
    minLength: number | null;
    maxLength: number | null;
    minStars: number | null;
    maxStars: number | null;
    minAr: number | null;
    maxAr: number | null;
    minOd: number | null;
    maxOd: number | null;
    minHp: number | null;
    maxHp: number | null;
    minCs: number | null;
    maxCs: number | null;
}

export const emptyModeFilters = (): ModeFilters => ({
    status: null,
    minLength: null, maxLength: null,
    minStars: null, maxStars: null,
    minAr: null, maxAr: null,
    minOd: null, maxOd: null,
    minHp: null, maxHp: null,
    minCs: null, maxCs: null,
});

export interface BeatmapFilters {
    excludedModes: number[];
    modeFilters: { [mode: number]: ModeFilters };
    lastPlayed: boolean | null;
}

interface BeatmapExporterState {
    filters: BeatmapFilters;
}

const initialState: BeatmapExporterState = {
    filters: {
        excludedModes: [],
        modeFilters: {},
        lastPlayed: null,
    },
};

export const BeatmapExporterStore = signalStore(
    { providedIn: 'root' },
    withState(initialState),

    withComputed((store, osuDb = inject(OsuDbStore)) => ({
        filteredBeatmaps: computed(() => {
            const { excludedModes, modeFilters, lastPlayed } = store.filters();

            return osuDb.beatmapSets().filter(set => {
                return set.diffs.some(d => {
                    if (lastPlayed !== null && d.lastPlayed !== lastPlayed) return false;
                    if (excludedModes.includes(d.mode)) return false;

                    const mf = modeFilters[d.mode];
                    if (mf) {
                        if (mf.status !== null && set.status !== mf.status) return false;
                        if (mf.minLength !== null && d.length < mf.minLength) return false;
                        if (mf.maxLength !== null && d.length > mf.maxLength) return false;
                        if (mf.minStars !== null && d.stars < mf.minStars) return false;
                        if (mf.maxStars !== null && d.stars > mf.maxStars) return false;
                        if (mf.minOd !== null && d.overallDifficulty < mf.minOd) return false;
                        if (mf.maxOd !== null && d.overallDifficulty > mf.maxOd) return false;
                        if (mf.minHp !== null && d.hpDrain < mf.minHp) return false;
                        if (mf.maxHp !== null && d.hpDrain > mf.maxHp) return false;
                        if (d.mode === 0 || d.mode === 2) {
                            if (mf.minAr !== null && d.approachRate < mf.minAr) return false;
                            if (mf.maxAr !== null && d.approachRate > mf.maxAr) return false;
                        }
                        if (mf.minCs !== null && d.circleSize < mf.minCs) return false;
                        if (mf.maxCs !== null && d.circleSize > mf.maxCs) return false;
                    }

                    return true;
                });
            });
        }),
    })),

    withComputed((store) => ({
        totalFiltered: computed(() => store.filteredBeatmaps().length),
    })),

    withMethods((store) => ({
        setFilters(filters: Partial<Omit<BeatmapFilters, 'modeFilters'>>) {
            patchState(store, { filters: { ...store.filters(), ...filters } });
        },
        setModeFilters(mode: number, filters: Partial<ModeFilters>) {
            const current = store.filters();
            patchState(store, {
                filters: {
                    ...current,
                    modeFilters: {
                        ...current.modeFilters,
                        [mode]: { ...(current.modeFilters[mode] ?? emptyModeFilters()), ...filters },
                    },
                },
            });
        },
        resetFilters() {
            patchState(store, { filters: initialState.filters });
        },
    }))
)
