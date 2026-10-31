export type BeatmapStatus = 'ranked' | 'approved' | 'qualified' | 'loved' | 'unranked' | 'unsubmitted' | 'unknown';

export interface OsuDiff {
    md5: string;
    diffName: string;
    mode: number;
    length: number;
    stars: number;
    lastPlayed: boolean;
    circleSize: number;
    approachRate: number;
    hpDrain: number;
    overallDifficulty: number;
    fileName: string;
    audio: string;
    creator: string;
    /** Per diff: a ranked set can hold diffs that were never uploaded, such as rates. */
    status: BeatmapStatus;
}

/** A folder of `Songs`. Unsubmitted sets have `beatmapsetId` 0, so `folderName` is what identifies a set. */
export interface OsuBeatmapSet {
    folderName: string;
    beatmapsetId: number;
    title: string;
    artist: string;
    diffs: OsuDiff[];
}

/** Whether the diff is on the osu! website, so it can be linked or downloaded. */
export function isSubmitted(diff: OsuDiff): boolean {
    return diff.status !== 'unsubmitted' && diff.status !== 'unknown';
}

const STATUS_PRIORITY: BeatmapStatus[] = ['ranked', 'approved', 'qualified', 'loved', 'unranked', 'unsubmitted', 'unknown'];

/** Status shown for a whole set: the best one among its diffs. */
export function setStatus(set: OsuBeatmapSet): BeatmapStatus {
    const statuses = new Set(set.diffs.map(d => d.status));
    return STATUS_PRIORITY.find(s => statuses.has(s)) ?? 'unknown';
}

/**
 * The sets of the diffs `md5s`, each with only those of its diffs, as a
 * collection shows them: diffs missing from `setsByMd5` are left out.
 */
export function setsOfDiffs(md5s: Iterable<string>, setsByMd5: ReadonlyMap<string, OsuBeatmapSet>): OsuBeatmapSet[] {
    const wanted = new Set(md5s);
    const seen = new Set<string>();
    const sets: OsuBeatmapSet[] = [];
    for (const md5 of wanted) {
        const set = setsByMd5.get(md5);
        if (!set || seen.has(set.folderName)) continue;
        seen.add(set.folderName);
        sets.push({ ...set, diffs: set.diffs.filter(d => wanted.has(d.md5)) });
    }
    return sets;
}
