import { inject } from "@angular/core";
import { ConfigurationStore, TrainerShortcut } from "@entities/configuration";
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

    withMethods((store, trainer = inject(TrainerStore), notify = inject(NotifyService), config = inject(ConfigurationStore)) => {
        async function write(custom: TrainerPreset[]) {
            await invoke('write_trainer_presets', { presets: custom });
            patchState(store, { custom });
        }

        /** Saves the shortcuts of the settings changed by `change`. */
        async function writeShortcuts(change: (shortcuts: TrainerShortcut[]) => TrainerShortcut[]) {
            const trainer = config.config()?.trainer;
            if (!trainer) return;
            await config.updateConfig({ trainer: { ...trainer, shortcuts: change(trainer.shortcuts ?? []) } });
        }

        return {
            /** Replaces the converts with the preset's. */
            apply(preset: TrainerPreset) {
                const skipped = trainer.setConverts(preset.converts);
                if (skipped > 0) {
                    notify.info(`${skipped} task(s) of "${preset.name}" don't work on this map's mode: they will be skipped.`);
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
                    await writeShortcuts(shortcuts => shortcuts.filter(s => s.preset !== name));
                } catch (e) {
                    notify.error(`Could not delete the preset: ${e}`);
                }
            },

            /** The shortcut of the preset called `preset`; empty, none. */
            shortcutOf(preset: string): string {
                return config.config()?.trainer.shortcuts?.find(s => s.preset === preset)?.keys ?? '';
            },

            /** Sets the shortcut of the preset called `preset`; empty keys take it away. */
            async setShortcut(preset: string, keys: string) {
                await writeShortcuts(shortcuts => {
                    const others = shortcuts.filter(s => s.preset !== preset);
                    return keys ? [...others, { preset, keys }] : others;
                });
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
