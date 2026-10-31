import { inject } from "@angular/core";
import { patchState, signalStore, withHooks, withMethods, withState } from "@ngrx/signals";
import { NotifyService } from "@shared/services";
import { invoke } from "@tauri-apps/api/core";
import { TrainerPreset } from "../models/trainer.model";
import { TrainerStore } from "./trainer.store";

interface TrainerPresetsState {
    /** The user's own presets, saved in `trainer-presets.json`. */
    custom: TrainerPreset[];
}

/** Whether `value`, read from the file, looks like a preset. */
function isPreset(value: unknown): value is TrainerPreset {
    const preset = value as TrainerPreset;
    return typeof preset?.name === 'string' && Array.isArray(preset.converts) && preset.converts.every(Array.isArray);
}

export const TrainerPresetsStore = signalStore(
    { providedIn: 'root' },
    withState<TrainerPresetsState>({ custom: [] }),

    withMethods((store, trainer = inject(TrainerStore), notify = inject(NotifyService)) => {
        async function write(custom: TrainerPreset[]) {
            await invoke('write_trainer_presets', { presets: custom });
            patchState(store, { custom });
        }

        return {
            /** Replaces the converts with the preset's. */
            apply(preset: TrainerPreset) {
                const dropped = trainer.setConverts(preset.converts);
                if (dropped > 0) {
                    notify.info(`${dropped} task(s) of "${preset.name}" don't work on this map's mode and were left out.`);
                }
            },

            /** Saves the converts as they are now, replacing the preset called `name` if there is one. */
            async saveCurrent(name: string) {
                const preset: TrainerPreset = { name, converts: trainer.converts().map(c => c.tasks.map(t => t.task)) };
                const replaced = store.custom().some(p => p.name === name);
                const custom = replaced ? store.custom().map(p => (p.name === name ? preset : p)) : [...store.custom(), preset];
                try {
                    await write(custom);
                    notify.success(replaced ? `Updated the preset "${name}"` : `Saved the preset "${name}"`);
                } catch (e) {
                    notify.error(`Could not save the preset: ${e}`);
                }
            },

            async remove(name: string) {
                try {
                    await write(store.custom().filter(p => p.name !== name));
                } catch (e) {
                    notify.error(`Could not delete the preset: ${e}`);
                }
            },
        };
    }),

    withHooks({
        onInit(store) {
            invoke<unknown>('read_trainer_presets')
                .then(presets => patchState(store, { custom: Array.isArray(presets) ? presets.filter(isPreset) : [] }))
                .catch(e => console.error('read_trainer_presets failed', e));
        },
    }),
);
