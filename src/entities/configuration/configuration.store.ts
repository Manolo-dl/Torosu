import { patchState, signalStore, withComputed, withMethods, withState } from "@ngrx/signals";
import { AppConfig } from "./configuration.model";
import { invoke } from "@tauri-apps/api/core";
import { computed } from "@angular/core";

interface ConfigState {
    config: AppConfig | null;
    isLoading: boolean;
    isLoaded: boolean,
}

const initialState: ConfigState = {
    config: null,
    isLoading: false,
    isLoaded: false,
};

export const ConfigurationStore = signalStore(
    { providedIn: 'root' },

    withState(initialState),

    withComputed((store) => ({
        /** Routes of the tools turned off in the configuration. */
        excludedTools: computed(() => new Set(store.config()?.excluded_tools ?? [])),
    })),

    withMethods((store) => ({
        async loadConfig() {
            patchState(store, { isLoading: true });
            
            try {
                const config = await invoke<AppConfig>('get_config');
                patchState(store, { config, isLoading: false, isLoaded: true });
            } catch {
                patchState(store, { isLoading: false, isLoaded: false });
            }
        },

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
        }

    }))
)