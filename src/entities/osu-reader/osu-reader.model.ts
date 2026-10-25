// Mirrors the payloads of src-tauri/src/commands/osu_reader/mod.rs and the
// serde types of the osu-memory-reader crate.

export type Ruleset = 'osu' | 'taiko' | 'fruits' | 'mania';

/** The beatmap selected in osu!, read from its memory. */
export interface OsuBeatmap {
    /** Ruleset selected in osu!, not the map's own; see `BeatmapAttributes.originalMode`. */
    mode: Ruleset | null;
    artist: string;
    title: string;
    creator: string;
    difficulty: string;
    ar: number;
    cs: number;
    hp: number;
    od: number;
    mapId: number;
    setId: number;
    rankedStatus: string;
    objectCount: number;
    folder: string;
    filename: string;
    audioFilename: string;
    backgroundFilename: string;
    /** MD5 of the `.osu` file. */
    checksum: string;
}

/** Full paths on disk of the selected beatmap. */
export interface BeatmapPaths {
    game: string;
    songs: string;
    folder: string;
    osu: string;
    background: string | null;
    audio: string | null;
}

/** Computed from the `.osu` file in the map's own mode, without converts or mods. */
export interface BeatmapAttributes {
    stars: number;
    bpm: { common: number; min: number; max: number };
    originalMode: Ruleset;
    mode: Ruleset;
    /** Only for osu!mania maps. */
    keyCount: number | null;
}

export interface SelectedBeatmap {
    beatmap: OsuBeatmap;
    /** `null` if the osu! folder was not found. */
    paths: BeatmapPaths | null;
    /** `null` if the `.osu` file could not be read. */
    attributes: BeatmapAttributes | null;
}

export type OsuReaderStatus =
    | { state: 'waiting' }
    | { state: 'connected' }
    | { state: 'error'; message: string };

export interface OsuReaderSnapshot {
    status: OsuReaderStatus;
    beatmap: SelectedBeatmap | null;
}
