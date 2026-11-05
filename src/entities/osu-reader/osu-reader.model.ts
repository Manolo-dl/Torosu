// Mirrors the payloads of src-tauri/src/commands/osu_reader/mod.rs and the
// serde types of the osu-memory-reader crate.

export type Ruleset = 'osu' | 'taiko' | 'fruits' | 'mania';

/** The beatmap selected in osu!, read from its memory. */
export interface OsuBeatmap {
    /** Ruleset selected in osu!, not the map's own; see `BeatmapAttributes.originalMode`. */
    mode: Ruleset | null;
    artist: string;
    /** Artist in its original language; empty when the map doesn't set one. */
    artistUnicode: string;
    title: string;
    /** Title in its original language; empty when the map doesn't set one. */
    titleUnicode: string;
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

/** A dan from Mania Tracker's calculator, e.g. `6++` (6.37). */
export interface Dan {
    /** Rice and LN dans use the same names (`1` ... `10`, `alpha` ... `zeta`). */
    family: 'rice' | 'ln';
    /** `6++`, `zeta-`. */
    name: string;
    /** The dan as a number, for comparing. */
    value: number;
    /** How sure the estimator is, from 0 to 1. */
    confidence: number;
}

/** Dans of a 4K, 6K or 7K mania map, at 1.0x and its own OD. */
export interface BeatmapDan {
    /** The one Mania Tracker shows first. */
    primary: Dan;
    rice: Dan | null;
    /** A hybrid map has both a rice and an LN dan. */
    ln: Dan | null;
}

/** Payload of `osu-beatmap-dan`, sent after the `osu-beatmap` of that map. */
export interface DanUpdate {
    checksum: string;
    dan: BeatmapDan;
}

export interface SelectedBeatmap {
    beatmap: OsuBeatmap;
    /** `null` if the osu! folder was not found. */
    paths: BeatmapPaths | null;
    /** `null` if the `.osu` file could not be read. */
    attributes: BeatmapAttributes | null;
    /** `null` until it arrives as `osu-beatmap-dan`, and for maps that can't be rated. */
    dan: BeatmapDan | null;
}

export type OsuReaderStatus =
    | { state: 'waiting' }
    | { state: 'connected' }
    | { state: 'error'; message: string };

export interface OsuReaderSnapshot {
    status: OsuReaderStatus;
    beatmap: SelectedBeatmap | null;
}
