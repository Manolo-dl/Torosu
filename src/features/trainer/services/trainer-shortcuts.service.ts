import { effect, inject, Injectable, signal, untracked } from '@angular/core';
import { invoke } from '@tauri-apps/api/core';
import { register, ShortcutEvent, unregister, unregisterAll } from '@tauri-apps/plugin-global-shortcut';
import { ConfigurationStore, TrainerShortcut } from '@entities/configuration';
import { OsuReaderStore } from '@entities/osu-reader';
import { NotifyService } from '@shared/services';
import { TranslocoService } from '@jsverse/transloco';
import { PlatformStore } from '@shared/stores';
import { applicableConverts, BUILT_IN_PRESETS } from '../models/trainer.model';
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
    private readonly transloco = inject(TranslocoService);

    /**
     * Whether a shortcut field is taking keys: the shortcuts are let go
     * meanwhile, or the system would keep their keys from the field (and run
     * them).
     */
    readonly paused = signal(false);
    /** How the system took each shortcut set, by its keys: none while paused. */
    readonly states = signal<Record<string, 'active' | 'failed'>>({});

    /** The registering under way: each change of the settings waits for the last. */
    private registering: Promise<void> = Promise.resolve();
    /** A convert started with a shortcut, still running. */
    private running = false;
    /**
     * Every shortcut registered since Torosu started: each is let go by its
     * keys before registering again. `unregisterAll` alone left some taken on
     * X11: the field of a deleted preset's keys never got them again, and a
     * new shortcut took two presses to set.
     */
    private readonly ever = new Set<string>();

    constructor() {
        effect(() => {
            const trainer = this.config.config()?.trainer;
            const on = !!trainer?.shortcuts_enabled && this.platform.wayland() === false;
            const paused = this.paused();
            const shortcuts = on && !paused ? (trainer?.shortcuts ?? []).filter(s => s.keys) : [];
            untracked(() => {
                // The map to convert is the one osu! has selected.
                if (on) this.reader.connect();
                this.registering = this.registering.then(() => this.register(shortcuts, !paused));
            });
        });
    }

    /** Lets go of every shortcut and takes `shortcuts`; with `report`, keeps how each went. */
    private async register(shortcuts: TrainerShortcut[], report: boolean) {
        if (report) log(`shortcuts: taking ${shortcuts.map(s => `${s.keys} (${s.preset})`).join(', ') || 'none'}`);
        try {
            await unregisterAll();
        } catch (e) {
            log(`shortcuts: unregisterAll failed: ${e}`);
        }
        for (const keys of this.ever) {
            try {
                await unregister(keys);
            } catch (e) {
                log(`shortcuts: unregister ${keys} failed: ${e}`);
            }
        }
        const states: Record<string, 'active' | 'failed'> = {};
        for (const shortcut of shortcuts) {
            this.ever.add(shortcut.keys);
            try {
                await register(shortcut.keys, event => this.run(event, shortcut.preset));
                states[shortcut.keys] = 'active';
            } catch (e) {
                // Another app has it, most of the time.
                states[shortcut.keys] = 'failed';
                log(`shortcuts: register ${shortcut.keys} failed: ${e}`);
            }
        }
        if (report) this.states.set(states);
    }

    private async run(event: ShortcutEvent, name: string) {
        if (event.state !== 'Pressed') return;
        log(`shortcuts: ${event.shortcut} pressed, for "${name}"`);

        const preset = [...BUILT_IN_PRESETS, ...this.presets.custom()].find(p => p.name === name);
        if (!preset) return this.tell(this.t('trainer.shortcuts.gone', { name }), 'error');
        if (this.running) return this.tell(this.t('trainer.shortcuts.running'), 'info');

        const data = this.reader.data();
        const osuFilePath = data?.paths?.osu;
        if (!this.reader.connected() || !data || !osuFilePath) {
            return this.tell(this.t('trainer.shortcuts.cantRead'), 'error');
        }
        const converts = applicableConverts(preset.converts, this.reader.originalMode());
        if (converts.length === 0) {
            return this.tell(this.t('trainer.shortcuts.noTasks', { name }), 'error');
        }

        this.running = true;
        const { title, difficulty, hp, cs, od, ar } = data.beatmap;
        this.tell(this.t('trainer.shortcuts.converting', { title, difficulty, name }), 'info');
        try {
            const names = await invoke<string[]>('run_trainer', {
                request: {
                    osuFilePath,
                    difficulty: { hp, cs, od, ar },
                    converts: converts.map(tasks => ({ tasks: tasks.map(task => ({ task })) })),
                },
            });
            this.tell(
                names.length === 1
                    ? this.t('trainer.created', { name: names[0] })
                    : this.t('trainer.createdMany', { count: names.length }),
                'success',
            );
        } catch (e) {
            this.tell(this.t('trainer.shortcuts.convertFailed', { error: String(e) }), 'error');
        } finally {
            this.running = false;
        }
    }

    /** Told over osu!, by the system, and in Torosu. */
    private t(key: string, params?: Record<string, unknown>): string {
        return this.transloco.translate(key, params);
    }

    private tell(message: string, kind: 'info' | 'success' | 'error') {
        invoke('show_notification', { title: this.t('trainer.shortcuts.notificationTitle'), body: message })
            .catch(e => console.error('show_notification failed', e));
        this.notify[kind](message);
    }
}

/** To the backend's log (see `log_frontend`): the webview's console is out of sight. */
function log(message: string) {
    invoke('log_frontend', { message }).catch(() => console.info(message));
}
