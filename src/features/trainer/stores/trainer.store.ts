import { errorText } from '@shared/i18n';
import { TranslocoService } from '@jsverse/transloco';
import { effect, inject, untracked } from "@angular/core";
import { ConfigurationStore } from "@entities/configuration";
import { OsuReaderStore, Skill } from "@entities/osu-reader";
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
    applicableConverts,
    skippedTasks,
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
    /**
     * The dan and MSD each convert would have, by its id (`null` if it can't
     * be rated); only for mania maps, and not with the setting on `none`.
     */
    previews: Record<string, Skill | null>;
    /** Whether `previews` are being estimated again; meanwhile they are the previous ones. */
    previewing: boolean;
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
    previews: {},
    previewing: false,
};

/** Wait after the last change before estimating the previews again, e.g. while a rate is dragged. */
const PREVIEW_DELAY_MS = 300;

export const TrainerStore = signalStore(
    { providedIn: 'root' },
    withState(initialState),

    withMethods((store, reader = inject(OsuReaderStore), notify = inject(NotifyService), transloco = inject(TranslocoService)) => {
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

            /**
             * Replaces the converts with `converts`, e.g. a preset's. Returns how
             * many of their tasks don't work on the map's mode: they stay, marked,
             * and are skipped when the maps are created.
             */
            setConverts(converts: TrainerTaskParams[][]): number {
                const next = converts.map(tasks =>
                    newConvert(tasks.map(task => ({ id: crypto.randomUUID(), task: structuredClone(task) })))
                );
                patchState(store, { converts: next.length > 0 ? next : [newConvert()] });
                return skippedTasks(converts, reader.originalMode());
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

                // The tasks for another mode are left out (see the marks on them).
                const converts = applicableConverts(store.converts().map(c => c.tasks.map(t => t.task)), reader.originalMode());
                if (converts.length === 0) {
                    notify.error(transloco.translate('trainer.noTasksForMode'));
                    return;
                }

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
                            converts: converts.map(tasks => ({ tasks: tasks.map(task => ({ task })) })),
                        },
                    });
                    notify.success(
                        names.length === 1
                            ? transloco.translate('trainer.created', { name: names[0] })
                            : transloco.translate('trainer.createdMany', { count: names.length }),
                    );
                } catch (e) {
                    notify.error(errorText(transloco, e));
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
            const config = inject(ConfigurationStore);

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
                    // osu!standard map) stay: the list marks them, and they are skipped.
                });
            });

            // The previews, again on every change of the map, the converts or the
            // OD (the only stat that changes a mania map's skill).
            let previewTimer: ReturnType<typeof setTimeout> | undefined;
            let previewRequest = 0;
            effect(() => {
                const osuFilePath = reader.data()?.paths?.osu;
                const mode = reader.originalMode();
                const display = config.config()?.trainer.skill_display ?? 'dan';
                const converts = store.converts();
                const od = store.od().value;

                clearTimeout(previewTimer);
                const request = ++previewRequest;
                if (!osuFilePath || mode !== 'mania' || display === 'none') {
                    untracked(() => patchState(store, { previews: {}, previewing: false }));
                    return;
                }

                untracked(() => patchState(store, { previewing: true }));
                previewTimer = setTimeout(async () => {
                    const difficulty = { hp: store.hp().value, cs: store.cs().value, od, ar: store.ar().value };
                    try {
                        const skills = await invoke<(Skill | null)[]>('preview_converts', {
                            request: {
                                osuFilePath,
                                difficulty,
                                // As `createMaps` sends them, but every convert, to keep the order.
                                converts: converts.map(c => ({
                                    tasks: c.tasks.filter(t => taskSupportsMode(t.task.type, mode)).map(t => ({ task: t.task })),
                                })),
                            },
                        });
                        if (request !== previewRequest) return;
                        patchState(store, {
                            previews: Object.fromEntries(converts.map((c, i) => [c.id, skills[i] ?? null])),
                            previewing: false,
                        });
                    } catch (e) {
                        if (request !== previewRequest) return;
                        console.warn('preview_converts failed', e);
                        patchState(store, { previews: {}, previewing: false });
                    }
                }, PREVIEW_DELAY_MS);
            });
        },
    }),
);
