import { effect, inject, untracked } from "@angular/core";
import { OsuReaderStore } from "@entities/osu-reader";
import { patchState, signalStore, withHooks, withMethods, withState } from "@ngrx/signals";
import { NotifyService } from "@shared/services";
import {
    DifficultyKey,
    DifficultyStat,
    FlnConverterParams,
    KeyConverterParams,
    RateChangeParams,
    TrainerConvert,
    TrainerTask,
    TrainerTaskParams,
    taskSupportsMode,
} from "../models/trainer.model";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface TrainerState {
    hp: DifficultyStat;
    cs: DifficultyStat;
    od: DifficultyStat;
    ar: DifficultyStat;
    /** Each one makes a diff, all at once; there is always at least one. */
    converts: TrainerConvert[];
    creating: boolean;
    progress: number;
    /** Whether the system ffmpeg is there, needed to scale the video. */
    ffmpegAvailable: boolean;
}

function newConvert(tasks: TrainerTask[] = []): TrainerConvert {
    return { id: crypto.randomUUID(), tasks };
}

const initialState: TrainerState = {
    hp: { value: 5, locked: false },
    cs: { value: 5, locked: false },
    od: { value: 5, locked: false },
    ar: { value: 5, locked: false },
    converts: [newConvert()],
    creating: false,
    progress: 0,
    ffmpegAvailable: false,
};

export const TrainerStore = signalStore(
    { providedIn: 'root' },
    withState(initialState),

    withMethods((store, reader = inject(OsuReaderStore), notify = inject(NotifyService)) => {
        /** Changes the tasks of convert `id`. */
        function updateTasks(id: string, change: (tasks: TrainerTask[]) => TrainerTask[]) {
            patchState(store, {
                converts: store.converts().map(c => c.id === id ? { ...c, tasks: change(c.tasks) } : c),
            });
        }

        return {
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

            addConvert() {
                patchState(store, { converts: [...store.converts(), newConvert()] });
            },

            /** A copy of convert `id` right after it, to change a setting or two. */
            duplicateConvert(id: string) {
                const converts = [...store.converts()];
                const index = converts.findIndex(c => c.id === id);
                if (index === -1) return;
                const tasks = converts[index].tasks.map(t => ({ ...t, id: crypto.randomUUID() }));
                converts.splice(index + 1, 0, newConvert(tasks));
                patchState(store, { converts });
            },

            removeConvert(id: string) {
                if (store.converts().length <= 1) return;
                patchState(store, { converts: store.converts().filter(c => c.id !== id) });
            },

            addTask(convertId: string, task: TrainerTaskParams) {
                updateTasks(convertId, tasks => [...tasks, { id: crypto.randomUUID(), task }]);
            },

            removeTask(convertId: string, taskId: string) {
                updateTasks(convertId, tasks => tasks.filter(t => t.id !== taskId));
            },

            updateTaskParams(convertId: string, taskId: string, params: RateChangeParams | FlnConverterParams | KeyConverterParams | {}) {
                updateTasks(convertId, tasks => tasks.map(t =>
                    t.id === taskId ? { ...t, task: { ...t.task, params } as TrainerTaskParams } : t
                ));
            },

            reorderTasks(convertId: string, previousIndex: number, currentIndex: number) {
                updateTasks(convertId, tasks => {
                    const reordered = [...tasks];
                    const [moved] = reordered.splice(previousIndex, 1);
                    reordered.splice(currentIndex, 0, moved);
                    return reordered;
                });
            },

            reset() {
                const beatmap = reader.data()?.beatmap;

                patchState(store, {
                    hp: { value : beatmap?.hp ?? 5, locked: false },
                    cs: { value : beatmap?.cs ?? 5, locked: false },
                    od: { value : beatmap?.od ?? 5, locked: false },
                    ar: { value : beatmap?.ar ?? 5, locked: false },
                    converts: [newConvert()],
                });
            },

            async createMaps() {
                const osuFilePath = reader.data()?.paths?.osu;
                if (!osuFilePath) return;

                patchState(store, { creating: true, progress: 0 });

                const unlisten = await listen<{ percent: number }>('trainer-progress', event => {
                    patchState(store, { progress: event.payload.percent });
                });

                try {
                    const names = await invoke<string[]>('run_trainer', {
                        request: {
                            osuFilePath,
                            difficulty: {
                                hp: store.hp().value,
                                cs: store.cs().value,
                                od: store.od().value,
                                ar: store.ar().value,
                            },
                            converts: store.converts().map(c => ({ tasks: c.tasks })),
                        },
                    });
                    notify.success(names.length === 1 ? `Created ${names[0]}` : `Created ${names.length} difficulties`);
                } catch (e) {
                    notify.error(String(e));
                } finally {
                    unlisten();
                    patchState(store, { creating: false });
                }
            },
        };
    }),

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
                    const converts = store.converts().map(c => ({
                        ...c,
                        tasks: c.tasks.filter(t => taskSupportsMode(t.task.type, mode)),
                    }));
                    if (converts.some((c, i) => c.tasks.length !== store.converts()[i].tasks.length)) {
                        patchState(store, { converts });
                    }
                });
            });
        },
    }),
);
