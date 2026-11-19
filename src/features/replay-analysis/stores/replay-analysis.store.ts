import { computed, effect, inject, untracked } from '@angular/core';
import { TranslocoService } from '@jsverse/transloco';
import { patchState, signalStore, withComputed, withHooks, withMethods, withState } from '@ngrx/signals';
import { invoke } from '@tauri-apps/api/core';
import { LocalScore, LocalScoresStore, ReplayAnalysis, maniaAccuracy, modAcronyms } from '@entities/local-scores';
import { ConfigurationStore, ReplaysConfig } from '@entities/configuration';
import { OsuBeatmapSet, OsuDbStore, OsuDiff } from '@entities/osu-db';
import { errorText } from '@shared/i18n';

/** A local score with its map, as the list shows it. */
export interface ScoreEntry {
    score: LocalScore;
    /** `null` if the map is no longer in osu!.db. */
    diff: OsuDiff | null;
    set: OsuBeatmapSet | null;
    keys: number | null;
    accuracy: number;
    mods: string[];
}

/**
 * What the summary and the charts time: everything osu! counts, only the
 * presses, or only the long notes (their presses and releases).
 */
export type TimingView = 'all' | 'presses' | 'ln';

interface ReplayAnalysisState {
    search: string;
    /** Only plays of this many keys; `null` for all. */
    keys: number | null;
    /** The replay (file name) analysed or being analysed. */
    selected: string | null;
    analysis: ReplayAnalysis | null;
    analyzing: boolean;
    error: string | null;
    view: TimingView;
}

const initialState: ReplayAnalysisState = {
    search: '',
    keys: null,
    selected: null,
    analysis: null,
    analyzing: false,
    error: null,
    view: 'all',
};

/** Until the configuration arrives, and in one written before the page: everything on. */
const DEFAULT_SETTINGS: ReplaysConfig = {
    follow: true,
    key_chart: true,
    histogram: true,
    timeline: true,
};

export const ReplayAnalysisStore = signalStore(
    { providedIn: 'root' },
    withState(initialState),

    withComputed((store, scores = inject(LocalScoresStore), osuDb = inject(OsuDbStore), config = inject(ConfigurationStore)) => {
        const entries = computed((): ScoreEntry[] => {
            const diffs = osuDb.diffsByMd5();
            const sets = osuDb.beatmapSetsByMd5();
            return scores.withReplay().map(score => {
                const diff = diffs.get(score.beatmapMd5) ?? null;
                return {
                    score,
                    diff,
                    set: sets.get(score.beatmapMd5) ?? null,
                    keys: diff ? Math.round(diff.circleSize) : null,
                    accuracy: maniaAccuracy(score.counts, score.mods),
                    mods: modAcronyms(score.mods),
                };
            });
        });

        return {
            entries,

            settings: computed((): ReplaysConfig => ({ ...DEFAULT_SETTINGS, ...config.config()?.replays })),

            /** The key counts there are plays of, for the filter. */
            keyCounts: computed(() =>
                [...new Set(entries().map(e => e.keys).filter((k): k is number => k !== null))].sort((a, b) => a - b),
            ),

            filtered: computed(() => {
                const search = store.search().trim().toLowerCase();
                const keys = store.keys();
                return entries().filter(entry =>
                    (keys === null || entry.keys === keys) &&
                    (!search || [entry.set?.title, entry.set?.artist, entry.diff?.diffName, entry.diff?.creator]
                        .some(text => text?.toLowerCase().includes(search))),
                );
            }),

            selectedEntry: computed(() => entries().find(e => e.score.replay === store.selected()) ?? null),
        };
    }),

    withMethods((store, transloco = inject(TranslocoService), config = inject(ConfigurationStore)) => ({
        setSearch(search: string) {
            patchState(store, { search });
        },

        setKeys(keys: number | null) {
            patchState(store, { keys });
        },

        setView(view: TimingView) {
            patchState(store, { view });
        },

        /** Saved in the configuration, like the page's other settings. */
        setFollow(follow: boolean) {
            config.updateConfig({ replays: { ...store.settings(), follow } });
        },

        /** Analyses the replay of `entry`, unless it is the one shown. */
        async select(entry: ScoreEntry) {
            const replay = entry.score.replay;
            if (!replay || !entry.set || !entry.diff || replay === store.selected()) return;

            patchState(store, { selected: replay, analysis: null, analyzing: true, error: null, view: 'all' });
            try {
                const analysis = await invoke<ReplayAnalysis>('analyze_replay', {
                    replay,
                    beatmap: `${entry.set.folderName}/${entry.diff.fileName}`,
                });
                // Another play may have been picked meanwhile.
                if (store.selected() === replay) patchState(store, { analysis, analyzing: false });
            } catch (error) {
                if (store.selected() === replay) {
                    patchState(store, { analyzing: false, error: errorText(transloco, error) });
                }
            }
        },
    })),

    withHooks({
        onInit(store, scores = inject(LocalScoresStore)) {
            // A play osu! just saved, analysed if the setting is on.
            effect(() => {
                const added = scores.added();
                untracked(() => {
                    if (!added || !store.settings().follow) return;
                    const entry = store.entries().find(e => e.score.replay === added.replay);
                    if (entry) store.select(entry);
                });
            });
        },
    }),
);
