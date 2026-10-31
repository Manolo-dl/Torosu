import type { OsuBeatmapSet, OsuDiff } from '@entities/osu-db';

export interface SelectedDiff {
    md5: string,
    folderName: string,
    fileName: string,
    audio: string
    newDiffName: string
}

export interface PackRequest {
    title: string,
    finalCreator: string,
    diffs: SelectedDiff[]
}

/** `diff` of `set` picked for the pack, named after its mapper, its song and itself. */
export function selectedDiff(set: OsuBeatmapSet, diff: OsuDiff): SelectedDiff {
    return {
        md5: diff.md5,
        folderName: set.folderName,
        fileName: diff.fileName,
        audio: diff.audio,
        newDiffName: `[${diff.creator}] ${set.artist} - ${set.title} (${diff.diffName})`,
    };
}
