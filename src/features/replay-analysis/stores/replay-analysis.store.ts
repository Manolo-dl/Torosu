import { computed, effect, inject, untracked } from '@angular/core';
import { TranslocoService } from '@jsverse/transloco';
import { patchState, signalStore, withComputed, withHooks, withMethods, withState } from '@ngrx/signals';
import { invoke } from '@tauri-apps/api/core';
import { LocalScore, LocalScoresStore, ReplayAnalysis, maniaAccuracy, modAcronyms } from '@entities/local-scores';
import { ConfigurationStore, ReplaysConfig } from '@entities/configuration';
import { OsuDbStore } from '@entities/osu-db';
import { OsuReaderStore } from '@entities/osu-reader';
import { errorText } from '@shared/i18n';

/** The map of a score, from osu!.db or from osu!'s memory. */
export interface ScoreMap {
    title: string;
    artist: string;
    diffName: string;
    creator: string;
    keys: number;
    /** The `.osu` file, as `<set folder>/<file>` in the songs folder. */
    path: string;
}

/** A local score with its map, as the list shows it. */
export interface ScoreEntry {
    score: LocalScore;
    /**
     * `null` if the map is neither in osu!.db nor was selected in osu! since
     * the page opened: osu! writes osu!.db only when it closes, so a map
     * downloaded meanwhile is found through the osu! reader.
     */
    map: ScoreMap | null;
    accuracy: number;
    mods: string[];
}

/**
 * What the summary and the charts time: everything osu! counts, only the
 * normal notes (rice), or only the long notes (their presses and releases).
 */
export type TimingView = 'all' | 'rice' | 'ln';

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
    /** The maps selected in osu! since the page opened, by MD5. */
    readMaps: Record<string, ScoreMap>;
}

const initialState: ReplayAnalysisState = {
    search: '',
    keys: null,
    selected: null,
    analysis: null,
    analyzing: false,
    error: null,
    view: 'all',
    readMaps: {},
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
            const read = store.readMaps();
            return scores.withReplay().map(score => {
                const diff = diffs.get(score.beatmapMd5);
                const set = sets.get(score.beatmapMd5);
                const map: ScoreMap | null = diff && set
                    ? {
                        title: set.title,
                        artist: set.artist,
                        diffName: diff.diffName,
                        creator: diff.creator,
                        keys: Math.round(diff.circleSize),
                        path: `${set.folderName}/${diff.fileName}`,
                    }
                    : read[score.beatmapMd5] ?? null;
                return { score, map, accuracy: maniaAccuracy(score.counts, score.mods), mods: modAcronyms(score.mods) };
            });
        });

        return {
            entries,

            settings: computed((): ReplaysConfig => ({ ...DEFAULT_SETTINGS, ...config.config()?.replays })),

            /** The key counts there are plays of, for the filter. */
            keyCounts: computed(() =>
                [...new Set(entries().flatMap(e => (e.map ? [e.map.keys] : [])))].sort((a, b) => a - b),
            ),

            filtered: computed(() => {
                const search = store.search().trim().toLowerCase();
                const keys = store.keys();
                return entries().filter(entry =>
                    (keys === null || entry.map?.keys === keys) &&
                    (!search || [entry.map?.title, entry.map?.artist, entry.map?.diffName, entry.map?.creator]
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
            if (!replay || !entry.map || replay === store.selected()) return;

            patchState(store, { selected: replay, analysis: null, analyzing: true, error: null, view: 'all' });
            try {
                const analysis = await invoke<ReplayAnalysis>('analyze_replay', {
                    replay,
                    beatmap: entry.map.path,
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
        onInit(store, scores = inject(LocalScoresStore), reader = inject(OsuReaderStore)) {
            // Each map selected in osu!, for those osu!.db doesn't have yet.
            effect(() => {
                const beatmap = reader.data()?.beatmap;
                if (!beatmap?.checksum || !beatmap.filename) return;
                const map: ScoreMap = {
                    title: beatmap.title,
                    artist: beatmap.artist,
                    diffName: beatmap.difficulty,
                    creator: beatmap.creator,
                    keys: Math.round(beatmap.cs),
                    path: `${beatmap.folder}/${beatmap.filename}`,
                };
                untracked(() => patchState(store, { readMaps: { ...store.readMaps(), [beatmap.checksum]: map } }));
            });

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
