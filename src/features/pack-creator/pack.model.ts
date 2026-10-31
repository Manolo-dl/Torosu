import type { OsuBeatmapSet, OsuDiff } from '@entities/osu-db';

export interface SelectedDiff {
    md5: string,
    folderName: string,
    fileName: string,
    audio: string
    newDiffName: string
}

/** `diff` of `set` picked for the pack, called `name` in it. */
export function selectedDiff(set: OsuBeatmapSet, diff: OsuDiff, name: string): SelectedDiff {
    return {
        md5: diff.md5,
        folderName: set.folderName,
        fileName: diff.fileName,
        audio: diff.audio,
        newDiffName: name,
    };
}

/** What the backend names a diff from (`PackDiff` in packs/naming.rs). */
export function nameSource(set: OsuBeatmapSet, diff: OsuDiff) {
    return {
        creator: diff.creator,
        artist: set.artist,
        title: set.title,
        diffName: diff.diffName,
        stars: diff.stars,
        mode: diff.mode,
        circleSize: diff.circleSize,
    };
}
