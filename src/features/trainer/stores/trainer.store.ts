import { effect, inject, untracked } from "@angular/core";
import { OsuReaderStore } from "@entities/osu-reader";
import { patchState, signalStore, withHooks, withMethods, withState } from "@ngrx/signals";
import { DifficultyKey, DifficultyStat, FlnConverterParams, KeyConverterParams, RateChangeParams, TrainerTask, TrainerTaskParams, taskSupportsMode } from "../models/trainer.model";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface TrainerState {
    hp: DifficultyStat;
    cs: DifficultyStat;
    od: DifficultyStat;
    ar: DifficultyStat;
    tasks: TrainerTask[];
    creating: boolean;
    progress: number;
    /** Whether the system ffmpeg is there, needed to scale the video. */
    ffmpegAvailable: boolean;
}

const initialState: TrainerState = {
    hp: { value: 5, locked: false },
    cs: { value: 5, locked: false },
    od: { value: 5, locked: false },
    ar: { value: 5, locked: false },
    tasks: [],
    creating: false,
    progress: 0,
    ffmpegAvailable: false,
};

export const TrainerStore = signalStore(
    { providedIn: 'root' },
    withState(initialState),

    withMethods((store, reader = inject(OsuReaderStore)) => ({

        setValue(stat: DifficultyKey, value: number) {
            patchState(store, {
                [stat]: { ...store[stat](), value }
            });
        },

        toggleLock(stat: DifficultyKey) {
            patchState(store, {
                [stat]: { ...store[stat](), locked: !store[stat]().locked }
            });
        },

        addTask(task: TrainerTaskParams) {
            const newTask: TrainerTask = { id: crypto.randomUUID(), task };
            patchState(store, { tasks: [...store.tasks(), newTask] });
        },

        removeTask(id: string) {
            patchState(store, { tasks: store.tasks().filter(task => task.id !== id) });
        },

        updateTaskParams(id: string, params: RateChangeParams | FlnConverterParams | KeyConverterParams | {}) {
            patchState(store, {
                tasks: store.tasks().map(t =>
                    t.id === id ? { ...t, task: { ...t.task, params } as TrainerTaskParams } : t
                ),
            });
        },

        reorderTasks(previousIndex: number, currentIndex: number) {
            const tasks = [...store.tasks()];
            const [moved] = tasks.splice(previousIndex, 1);
            tasks.splice(currentIndex, 0, moved);
            patchState(store, { tasks });
        },

        reset() {
            const beatmap = reader.data()?.beatmap;

            patchState(store, {
                hp: { value : beatmap?.hp ?? 5, locked: false },
                cs: { value : beatmap?.cs ?? 5, locked: false },
                od: { value : beatmap?.od ?? 5, locked: false },
                ar: { value : beatmap?.ar ?? 5, locked: false },
                tasks: [],
            });
        },

        async createMap() {
            const osuFilePath = reader.data()?.paths?.osu;
            if (!osuFilePath) return;

            patchState(store, { creating: true, progress: 0 });

            const unlisten = await listen<{ percent: number }>('trainer-progress', event => {
                patchState(store, { progress: event.payload.percent });
            });

            invoke('run_trainer', {
                request: {
                    osuFilePath,
                    difficulty: {
                        hp: store.hp().value,
                        cs: store.cs().value,
                        od: store.od().value,
                        ar: store.ar().value,
                    },
                    tasks: store.tasks(),
                },
            }).catch((e) => {
                console.error('run_trainer failed', e);
            }).finally(() => {
                unlisten();
                patchState(store, { creating: false });
            });
        }
    })),

    withHooks({
        onInit(store) {
            const reader = inject(OsuReaderStore);

            invoke<boolean>('is_ffmpeg_available')
                .then(ffmpegAvailable => patchState(store, { ffmpegAvailable }))
                .catch(() => patchState(store, { ffmpegAvailable: false }));

            let lastBeatmapId: string | null = null;

            effect(() => {
                const data = reader.data();
                if (!data) return;

                const currentBeatmapId = data.beatmap.checksum;

                if (currentBeatmapId === lastBeatmapId) return;
                lastBeatmapId = currentBeatmapId;

                const beatmap = data.beatmap;
                const mode = data.attributes?.originalMode ?? null;

                untracked(() => {
                    (['hp', 'cs', 'ar', 'od'] as const).forEach((stat) => {

                        const current = store[stat]();
                        const newValue = beatmap[stat];

                        if (!current.locked && current.value !== newValue) {
                            patchState(store, {
                                [stat]: { ...current, value: newValue }
                            });
                        }
                    });

                    // Tasks for another mode (e.g. the key converter after picking an
                    // osu!standard map) can't run on this map.
                    const tasks = store.tasks().filter(t => taskSupportsMode(t.task.type, mode));
                    if (tasks.length !== store.tasks().length) patchState(store, { tasks });
                });
            });
        },
    }),
);