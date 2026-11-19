/** Judgement counts of a mania play, with osu! stable's names in mania. */
export interface ManiaCounts {
    /** MAX (geki). */
    max: number;
    n300: number;
    /** 200 (katu). */
    n200: number;
    n100: number;
    n50: number;
    miss: number;
}

/** A mania play of `scores.db` (`list_local_scores`). */
export interface LocalScore {
    beatmapMd5: string;
    playerName: string;
    /** osu!'s mod bits. */
    mods: number;
    score: number;
    maxCombo: number;
    perfect: boolean;
    counts: ManiaCounts;
    /** When it was set, in milliseconds since the Unix epoch. */
    timestamp: number;
    /** 0 if it was not submitted. */
    onlineId: number;
    /** File name of its replay in `Data/r`; `null` if osu! has none. */
    replay: string | null;
}

export type Judgement = 'max' | 'n300' | 'n200' | 'n100' | 'n50' | 'miss';

export const JUDGEMENTS: Judgement[] = ['max', 'n300', 'n200', 'n100', 'n50', 'miss'];

/** How far from a note each judgement reaches, in real time (ms). */
export interface HitWindows {
    max: number;
    n300: number;
    n200: number;
    n100: number;
    n50: number;
    miss: number;
}

/** How a set of hits or releases was timed (ms; negative is early). */
export interface Timing {
    count: number;
    mean: number;
    median: number;
    /** Ten times the standard deviation. */
    unstableRate: number;
    early: number;
    late: number;
    /** Mean of the early ones and of the rest, as osu!'s results screen shows them. */
    earlyMean: number;
    lateMean: number;
}

export interface KeyStats {
    key: number;
    notes: number;
    counts: ManiaCounts;
    /** Presses that hit a note within the 50 window. */
    hits: Timing;
    /** Releases of long notes within the 50 window of their end. */
    releases: Timing;
    /** Long notes let go before the 50 window of their end. */
    earlyReleases: number;
    /** Long notes held past the 50 window of their end. */
    lateReleases: number;
}

/** One note as it was played. Times are the song's, in ms. */
export interface NoteEvent {
    time: number;
    key: number;
    judgement: Judgement;
    /** With ScoreV2, of the end of a long note. */
    tail: Judgement | null;
    /** ms from the note; `null` if it wasn't pressed. */
    hit: number | null;
    /** ms from the end of a long note when it was let go. */
    release: number | null;
}

export interface ReplayStats {
    counts: ManiaCounts;
    hits: Timing;
    releases: Timing;
    /** Presses and releases together: what osu!'s results screen counts. */
    combined: Timing;
    keys: KeyStats[];
    notes: NoteEvent[];
}

/** What a replay says about how its map was played (`analyze_replay`). */
export interface ReplayAnalysis {
    beatmapMd5: string;
    playerName: string;
    mods: number;
    keys: number;
    od: number;
    rate: number;
    windows: HitWindows;
    /** Stable writes the scroll speed in the first frames, scaled by the BPM. */
    scrollSpeedScale: number | null;
    /** What the game counted. `stats.counts` are the analysis' own. */
    gameCounts: ManiaCounts;
    /** When the game counted each miss (ms, song time). */
    gameMisses: number[];
    stats: ReplayStats;
}

const MOD_ACRONYMS: [number, string][] = [
    [1 << 0, 'NF'],
    [1 << 1, 'EZ'],
    [1 << 3, 'HD'],
    [1 << 4, 'HR'],
    [1 << 5, 'SD'],
    [1 << 14, 'PF'],
    [1 << 9, 'NC'],
    [1 << 6, 'DT'],
    [1 << 8, 'HT'],
    [1 << 10, 'FL'],
    [1 << 20, 'FI'],
    [1 << 21, 'RD'],
    [1 << 26, 'CP'],
    [1 << 30, 'MR'],
    [1 << 29, 'V2'],
];

/** The mods of `bits`, as osu! names them (`HDDT`); NC hides the DT it comes with, PF the SD. */
export function modAcronyms(bits: number): string[] {
    const hidden = (bits & (1 << 9) ? 1 << 6 : 0) | (bits & (1 << 14) ? 1 << 5 : 0);
    return MOD_ACRONYMS.filter(([bit]) => bits & bit && !(hidden & bit)).map(([, name]) => name);
}

export function totalJudgements(counts: ManiaCounts): number {
    return counts.max + counts.n300 + counts.n200 + counts.n100 + counts.n50 + counts.miss;
}

/** Accuracy in percent, with stable's formula; ScoreV2 weighs MAX higher. */
export function maniaAccuracy(counts: ManiaCounts, mods = 0): number {
    const total = totalJudgements(counts);
    if (total === 0) return 100;
    const max = mods & (1 << 29) ? 305 : 300;
    const points = counts.max * max + counts.n300 * 300 + counts.n200 * 200 + counts.n100 * 100 + counts.n50 * 50;
    return (points / (total * max)) * 100;
}
