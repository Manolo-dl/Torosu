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
    /** The dan without its variant: `6`, `zeta`, `azimuth` (7K), `terra` (6K). */
    label: string;
    /** `++`, `+`, `-` or `--`; `null` for the dan itself. */
    variant: string | null;
    /** The dan as a number, for comparing. */
    value: number;
    /** How sure the estimator is, from 0 to 1. */
    confidence: number;
}

/** The dans of a 4K, 6K or 7K mania map. */
export interface Dans {
    /** The one Mania Tracker shows first. */
    primary: Dan;
    rice: Dan | null;
    /** A hybrid map has both a rice and an LN dan. */
    ln: Dan | null;
}

/** A map's MSD: MinaCalc's, or Prism's on 6K-8K. */
export interface Msd {
    /** Each gives its own scale: two MSDs only compare when the same one gives both. */
    calculator: 'minaCalc' | 'prism';
    overall: number;
    /** Every skillset but the overall (`Stream`, `Jumpstream`, ... `LN`), in MinaCalc's order. */
    skillsets: { name: string; value: number }[];
}

/** How hard a mania map is, from Mania Tracker's calculator; a 5K map has an MSD but no dan. */
export interface Skill {
    /** Of the map rated: a convert can change it. */
    keyCount: number;
    dan: Dans | null;
    msd: Msd | null;
}

/** Payload of `osu-beatmap-skill`, sent after the `osu-beatmap` of that map. */
export interface SkillUpdate {
    checksum: string;
    skill: Skill;
}

/** The mods selected in osu! that change how hard a map is: DT/NC, HT, HR, EZ. */
export interface PlayedMods {
    /** Their acronyms as osu! shows them: `DT`, `HTHR`. */
    name: string;
    /** 1.5 with DT/NC, 0.75 with HT, else 1. */
    rate: number;
}

/** A map's skill with the mods selected in osu!. */
export interface PlayedSkill {
    mods: PlayedMods;
    skill: Skill;
}

/** Payload of `osu-beatmap-played-skill`: `null` with no mod that changes the skill, or the setting off. */
export interface PlayedSkillUpdate {
    checksum: string;
    played: PlayedSkill | null;
}

export interface SelectedBeatmap {
    beatmap: OsuBeatmap;
    /** `null` if the osu! folder was not found. */
    paths: BeatmapPaths | null;
    /** `null` if the `.osu` file could not be read. */
    attributes: BeatmapAttributes | null;
    /** At 1.0x: `null` until it arrives as `osu-beatmap-skill`, and for maps that can't be rated. */
    skill: Skill | null;
    /** With the mods selected in osu!, while `include_mods` is on and they change it. */
    played: PlayedSkill | null;
}

export type OsuReaderStatus =
    | { state: 'waiting' }
    | { state: 'connected' }
    | { state: 'error'; message: string };

export interface OsuReaderSnapshot {
    status: OsuReaderStatus;
    beatmap: SelectedBeatmap | null;
    /** osu! is playing a map (or a replay). */
    inPlay: boolean;
}
