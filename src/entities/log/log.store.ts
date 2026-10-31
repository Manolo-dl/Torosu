import { signalStore, withComputed } from "@ngrx/signals";
import { LogEntry } from "./log.model";
import { computed } from "@angular/core";
import { withLoader } from "@shared/stores";
import { invoke } from "@tauri-apps/api/core";

interface LogState {
    logs: LogEntry[];
}

const initialState: LogState = {
    logs: [],
};

export const LogStore = signalStore(
    { providedIn: 'root' },
    withLoader(initialState, {
        read: () => invoke<LogEntry[]>('read_logs'),
        // Newest first.
        loaded: logs => ({
            logs: [...logs].sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime()),
        }),
        failure: 'Failed to load logs',
    }),

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
)
