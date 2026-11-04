import { effect, inject, Injectable, untracked } from '@angular/core';
import { invoke } from '@tauri-apps/api/core';
import { register, ShortcutEvent, unregisterAll } from '@tauri-apps/plugin-global-shortcut';
import { ConfigurationStore, TrainerShortcut } from '@entities/configuration';
import { OsuReaderStore } from '@entities/osu-reader';
import { NotifyService } from '@shared/services';
import { PlatformStore } from '@shared/stores';
import { applicableConverts, BUILT_IN_PRESETS } from '../models/trainer.model';
import { shortcutLabel } from '../models/shortcut';
import { TrainerPresetsStore } from '../stores/trainer-presets.store';

/**
 * The keyboard shortcuts that run a preset on the map selected in osu!, while
 * the game has the focus: registered with the system as the settings say, off
 * by default, and never on Wayland, where an app can't listen to the keyboard
 * of another window. The tasks for another mode than the map's are skipped,
 * and the map keeps its own difficulty settings.
 */
@Injectable({ providedIn: 'root' })
export class TrainerShortcutsService {
    private readonly config = inject(ConfigurationStore);
    private readonly platform = inject(PlatformStore);
    private readonly reader = inject(OsuReaderStore);
    private readonly presets = inject(TrainerPresetsStore);
    private readonly notify = inject(NotifyService);

    /** The registering under way: each change of the settings waits for the last. */
    private registering: Promise<void> = Promise.resolve();
    /** A convert started with a shortcut, still running. */
    private running = false;

    constructor() {
        effect(() => {
            const trainer = this.config.config()?.trainer;
            const on = !!trainer?.shortcuts_enabled && this.platform.wayland() === false;
            const shortcuts = on ? (trainer?.shortcuts ?? []).filter(s => s.keys) : [];
            untracked(() => {
                // The map to convert is the one osu! has selected.
                if (on) this.reader.connect();
                this.registering = this.registering.then(() => this.register(shortcuts));
            });
        });
    }

    private async register(shortcuts: TrainerShortcut[]) {
        try {
            await unregisterAll();
        } catch (e) {
            console.error('unregisterAll failed', e);
        }
        for (const shortcut of shortcuts) {
            try {
                await register(shortcut.keys, event => this.run(event, shortcut.preset));
            } catch (e) {
                // Another app has it, most of the time.
                this.notify.error(`Could not set ${shortcutLabel(shortcut.keys)} for "${shortcut.preset}": ${e}`);
            }
        }
    }

    private async run(event: ShortcutEvent, name: string) {
        if (event.state !== 'Pressed') return;

        const preset = [...BUILT_IN_PRESETS, ...this.presets.custom()].find(p => p.name === name);
        if (!preset) return this.tell(`The preset "${name}" is gone.`, 'error');
        if (this.running) return this.tell('The last convert is still running.', 'info');

        const data = this.reader.data();
        const osuFilePath = data?.paths?.osu;
        if (!this.reader.connected() || !data || !osuFilePath) {
            return this.tell("Can't read the map selected in osu!.", 'error');
        }
        const converts = applicableConverts(preset.converts, this.reader.originalMode());
        if (converts.length === 0) {
            return this.tell(`None of the tasks of "${name}" work on this map's mode.`, 'error');
        }

        this.running = true;
        const { title, difficulty, hp, cs, od, ar } = data.beatmap;
        this.tell(`Converting ${title} [${difficulty}] with "${name}"…`, 'info');
        try {
            const names = await invoke<string[]>('run_trainer', {
                request: {
                    osuFilePath,
                    difficulty: { hp, cs, od, ar },
                    converts: converts.map(tasks => ({ tasks: tasks.map(task => ({ task })) })),
                },
            });
            this.tell(names.length === 1 ? `Created ${names[0]}` : `Created ${names.length} difficulties`, 'success');
        } catch (e) {
            this.tell(`The convert failed: ${e}`, 'error');
        } finally {
            this.running = false;
        }
    }

    /** Told over osu!, by the system, and in Torosu. */
    private tell(message: string, kind: 'info' | 'success' | 'error') {
        invoke('show_notification', { title: 'Torosu trainer', body: message })
            .catch(e => console.error('show_notification failed', e));
        this.notify[kind](message);
    }
}
