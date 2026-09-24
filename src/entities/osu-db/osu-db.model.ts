export interface OsuDiff {
    md5: string;
    diffName: string;
    mode: number;
    bpm: number;
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
}

export interface OsuBeatmapSet {
    beatmapsetId: number;
    title: string;
    artist: string;
    status: string;
    diffs: OsuDiff[];
}