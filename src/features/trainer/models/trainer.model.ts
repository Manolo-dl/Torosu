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

/** Playstyles renamed, as old name to new one (see `RENAMED` in `analysis/layout.rs`). */
const RENAMED_PLAYSTYLES: Record<string, string> = { '1+7': '7K+1' };

/** `tasks` with the key converter's playstyle under its current name: presets saved before keep theirs. */
export function renamePlaystyles(tasks: TrainerTaskParams[]): TrainerTaskParams[] {
    return tasks.map(task => {
        if (task.type !== 'KeyConverter') return task;
        const renamed = task.params.playstyle ? RENAMED_PLAYSTYLES[task.params.playstyle] : undefined;
        return renamed ? { ...task, params: { ...task.params, playstyle: renamed } } : task;
    });
}

/**
 * The playstyles of each key count, the usual one first (see `analysis/layout.rs`).
 * The name is sent to the backend and goes in the diff name as it is; the hint
 * is a translation key.
 */
export const PLAYSTYLES: Record<number, { name: string; hint: string }[]> = {
    3: [
        { name: '1+1+1', hint: 'trainer.playstyles.3_1+1+1' },
        { name: '2+1', hint: 'trainer.playstyles.3_2+1' },
    ],
    5: [
        { name: '2+1+2', hint: 'trainer.playstyles.5_2+1+2' },
        { name: '3+2', hint: 'trainer.playstyles.5_3+2' },
    ],
    8: [
        { name: '4+4', hint: 'trainer.playstyles.8_4+4' },
        { name: '3+2+3', hint: 'trainer.playstyles.8_3+2+3' },
        { name: '7K+1', hint: 'trainer.playstyles.8_7K+1' },
    ],
    // 12K to 18K: the playstyles of the osu! wiki (osu!mania 10K+ playstyles).
    12: [
        { name: '12K', hint: 'trainer.playstyles.12_12K' },
        { name: '10K2S', hint: 'trainer.playstyles.12_10K2S' },
    ],
    14: [
        { name: 'DP', hint: 'trainer.playstyles.14_DP' },
        { name: 'EZ2AC', hint: 'trainer.playstyles.14_EZ2AC' },
    ],
    16: [
        { name: 'DP', hint: 'trainer.playstyles.16_DP' },
        { name: 'EZ2AC', hint: 'trainer.playstyles.16_EZ2AC' },
    ],
    18: [
        { name: '10K8K', hint: 'trainer.playstyles.18_10K8K' },
        { name: '9K9K', hint: 'trainer.playstyles.18_9K9K' },
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

/** A diff to generate: its own tasks, applied in order. The difficulty settings are shared. */
export interface TrainerConvert {
    id: string;
    tasks: TrainerTask[];
}

/** A list of converts to start from, each with its tasks. */
export interface TrainerPreset {
    name: string;
    converts: TrainerTaskParams[][];
}

/** A rate change with the default options. */
export function rateChangeTask(rate: number): TrainerTaskParams {
    return { type: 'RateChange', params: { rate, adjustPitch: false, scaleStoryboard: false, scaleVideo: false } };
}

/** Presets that come with the trainer; the user's own are saved next to the settings. */
export const BUILT_IN_PRESETS: TrainerPreset[] = [
    { name: 'Rates 1.1x – 1.4x', converts: [1.1, 1.2, 1.3, 1.4].map(rate => [rateChangeTask(rate)]) },
];

/** Translation keys. */
export const TASK_LABELS: Record<TrainerTaskType, string> = {
    RateChange: 'trainer.tasks.RateChange',
    SVNormalizer: 'trainer.tasks.SVNormalizer',
    FlnConverter: 'trainer.tasks.FlnConverter',
    KeyConverter: 'trainer.tasks.KeyConverter',
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

/**
 * The converts as they run on a map made for `mode`: without the tasks for
 * another mode, which stay in the list but are not applied, and without the
 * converts left with none of their tasks (they would only copy the map).
 */
export function applicableConverts(converts: TrainerTaskParams[][], mode: Ruleset | null): TrainerTaskParams[][] {
    return converts
        .map(tasks => ({ tasks, kept: tasks.filter(task => taskSupportsMode(task.type, mode)) }))
        .filter(({ tasks, kept }) => tasks.length === 0 || kept.length > 0)
        .map(({ kept }) => kept);
}

/** How many tasks of `converts` don't apply to a map made for `mode`. */
export function skippedTasks(converts: TrainerTaskParams[][], mode: Ruleset | null): number {
    return converts.flat().filter(task => !taskSupportsMode(task.type, mode)).length;
}

export interface DifficultyStat {
    value: number;
    locked: boolean;
}

