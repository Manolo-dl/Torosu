export type DifficultyKey = 'hp' | 'cs' | 'od' | 'ar';

export type TrainerTaskType = 'RateChange' | 'Mirror' | 'RemoveSV' | 'NoSpinner';

export interface RateChangeParams {
    rate: number;
    adjustPitch: boolean;
}

export interface MirrorParams {
    axis: 'horizontal' | 'vertical';
}

export type TrainerTaskParams =
    | { type: 'RateChange'; params: RateChangeParams }
    | { type: 'Mirror'; params: MirrorParams }
    | { type: 'RemoveSV'; params: {} }
    | { type: 'NoSpinner'; params: {} };

export interface TrainerTask {
    id: string;
    task: TrainerTaskParams;
}

export const TASK_LABELS: Record<TrainerTaskType, string> = {
    RateChange: 'Rate Change',
    Mirror: 'Mirror',
    RemoveSV: 'Remove SV',
    NoSpinner: 'No Spinner',
};

export interface DifficultyStat {
    value: number;
    locked: boolean;
}

