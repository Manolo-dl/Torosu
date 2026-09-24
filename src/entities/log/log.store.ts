import { patchState, signalStore, withComputed, withMethods, withState } from "@ngrx/signals";
import { LogEntry } from "./log.model";
import { computed, inject } from "@angular/core";
import { ToastStore } from "@shared/stores";
import { invoke } from "@tauri-apps/api/core";

interface LogState {
    logs: LogEntry[];
    isLoaded: boolean;
}

const initialState: LogState = {
    logs: [],
    isLoaded: false,
};

export const LogStore = signalStore(
    { providedIn: 'root' },
    withState(initialState),

    withComputed((store) => ({
        totalLogs: computed(() => store.logs().length),
        groupedLogs: computed(() => {
            const result: Record<string, Record<string, LogEntry[]>> = {
                DEBUG: {},
                INFO: {},
                WARN: {},
                ERROR: {},
            };

            for (const entry of store.logs()) {
                const level = entry.level.toUpperCase();
                if (!result[level]) result[level] = {};
                if (!result[level][entry.target]) result[level][entry.target] = [];
                result[level][entry.target].push(entry);
            }

            return result;
        }),
    })),

    withMethods((store, toast = inject(ToastStore)) => ({
        async load() {
            try {
                const logs = await invoke<LogEntry[]>('read_logs');
                const sorted = [...logs].sort((a, b) =>
                    new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime()
                );
                patchState(store, { logs: sorted, isLoaded: true });
            } catch (error) {
                toast.show('error', 'Failed to load logs');
                patchState(store, { isLoaded: false });
                throw error;
            }
        }
    })),
)