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
export type KeyDensity = 'none' | 'dense' | 'moreDense';

/** What the key converter favours: the original shapes, or every key evenly. */
export type PatternStyle = 'faithful' | 'balanced' | 'spread';

export interface KeyConverterParams {
    targetKeys: number;
    density: KeyDensity;
    style: PatternStyle;
    /** Keeps the map's jacks on a single key instead of splitting them. */
    keepJacks: boolean;
}

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

export interface DifficultyStat {
    value: number;
    locked: boolean;
}

