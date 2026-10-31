import { patchState, signalStore, withComputed, withMethods } from "@ngrx/signals";
import { AppConfig } from "./configuration.model";
import { invoke } from "@tauri-apps/api/core";
import { computed } from "@angular/core";
import { withLoader } from "@shared/stores";

interface ConfigState {
    config: AppConfig | null;
}

const initialState: ConfigState = {
    config: null,
};

export const ConfigurationStore = signalStore(
    { providedIn: 'root' },

    // The app asks for it at startup; the guards and pages that need it
    // before it arrives wait for that same request (`ensureLoaded`).
    withLoader(initialState, {
        read: () => invoke<AppConfig>('get_config'),
        loaded: config => ({ config }),
        // The guards wait for it: if it fails they go on with the defaults
        // instead of blocking the navigation.
        rethrow: false,
    }),

    withComputed((store) => ({
        /** Routes of the tools turned off in the configuration. */
        excludedTools: computed(() => new Set(store.config()?.excluded_tools ?? [])),
    })),

    withMethods((store) => ({
        async updateConfig(partial: Partial<AppConfig>) {
            const current = store.config();
            if (!current) return;

            const updated: AppConfig = { ...current, ...partial };

            try {
                await invoke('update_config', { config: updated });
                patchState(store, { config: updated });
            } catch {
                // Handle error if needed
            }
        },
    })),
)
