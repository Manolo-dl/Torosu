import type { Ruleset } from '@entities/osu-reader';

export type DifficultyKey = 'hp' | 'cs' | 'od' | 'ar';

export type TrainerTaskType = 'RateChange' | 'SVNormalizer' | 'FlnConverter' | 'KeyConverter';

export interface RateChangeParams {
    rate: number;
    adjustPitch: boolean;
    /** Also scales the shared .osb; the diff is then exported as a new set. */
    scaleStoryboard: boolean;
    /** Re-encodes the video at the new speed; needs ffmpeg in the PATH. */
    scaleVideo: boolean;
}

/** A length in beats (`1/value` of a beat) or in milliseconds. */
export interface NoteLength {
    unit: 'beat' | 'ms';
    value: number;
}

export interface FlnConverterParams {
    /** Time each long note is released before the next note in its column. */
    gap: NoteLength;
    /** Long notes shorter than this stay as normal notes. */
    minLength: NoteLength;
    /** Also reshapes the long notes the map already has. */
    reworkExistingLn: boolean;
}

/** Extra notes added to fill the new keys when converting to more keys. */
export type KeyDensity = 'none' | 'dense' | 'moreDense' | 'full';

export interface KeyConverterParams {
    targetKeys: number;
    density: KeyDensity;
    /** Keeps the map's jacks on a single key instead of splitting them. */
    keepJacks: boolean;
    /** How the target keys are played; `null` for the usual playstyle of the key count. */
    playstyle: string | null;
}

/** The playstyles of each key count, the usual one first (see `analysis/layout.rs`). */
export const PLAYSTYLES: Record<number, { name: string; hint: string }[]> = {
    3: [
        { name: '1+1+1', hint: 'Indexes and a thumb.' },
        { name: '2+1', hint: 'Two keys on the left hand, one on the right.' },
    ],
    5: [
        { name: '2+1+2', hint: 'Thumb on the middle key.' },
        { name: '3+2', hint: 'Middle key with the left index.' },
        { name: '2+3', hint: 'Middle key with the right index.' },
    ],
    6: [
        { name: '3+3', hint: 'Three fingers per hand.' },
        { name: '2+1+1+2', hint: 'Both thumbs on the middle keys.' },
    ],
    8: [
        { name: '4+4', hint: 'Four fingers per hand.' },
        { name: '3+2+3', hint: 'Both thumbs on the middle keys.' },
        { name: '1+7', hint: '7K with a scratch on the left, IIDX style.' },
        { name: '7+1', hint: '7K with a scratch on the right.' },
    ],
    9: [
        { name: '4+1+4', hint: "Thumb on the middle key, pop'n style." },
        { name: '5+4', hint: 'Middle key with the left thumb.' },
        { name: '4+5', hint: 'Middle key with the right thumb.' },
    ],
};

export type TrainerTaskParams =
    | { type: 'RateChange'; params: RateChangeParams }
    | { type: 'SVNormalizer'; params: {} }
    | { type: 'FlnConverter'; params: FlnConverterParams }
    | { type: 'KeyConverter'; params: KeyConverterParams };

export interface TrainerTask {
    id: string;
    task: TrainerTaskParams;
}

export const TASK_LABELS: Record<TrainerTaskType, string> = {
    RateChange: 'Rate Change',
    SVNormalizer: 'SV Normalizer',
    FlnConverter: 'FLN Converter',
    KeyConverter: 'Key Converter',
};

/** Modes each task works on. The map's own mode counts, not osu!'s automatic converts. */
export const TASK_MODES: Record<TrainerTaskType, readonly Ruleset[] | 'all'> = {
    RateChange: 'all',
    SVNormalizer: 'all',
    FlnConverter: ['mania'],
    KeyConverter: ['mania'],
};

/** Whether a task can run on a map made for `mode`; `null` (unknown mode) only allows the ones for every mode. */
export function taskSupportsMode(type: TrainerTaskType, mode: Ruleset | null): boolean {
    const modes = TASK_MODES[type];
    return modes === 'all' || (mode !== null && modes.includes(mode));
}

export interface DifficultyStat {
    value: number;
    locked: boolean;
}

