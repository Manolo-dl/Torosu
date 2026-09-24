import { patchState, signalStore, withComputed, withHooks, withMethods, withState } from "@ngrx/signals";
import { DatabaseColumn, DatabaseTable } from "./database.model";
import { computed, effect, inject } from "@angular/core";
import { ToastStore } from "@shared/stores";
import { invoke } from "@tauri-apps/api/core";

interface DatabaseState {
    tables: DatabaseTable[];
    isLoaded: boolean;
    selectedTable: string | null;
    excludedColumns: string[];
    results: Record<string, unknown[]>;
    isQuerying: boolean;
}

const initialState: DatabaseState = {
    tables: [],
    isLoaded: false,
    selectedTable: null,
    excludedColumns: [],
    results: {},
    isQuerying: false
};

export const DatabaseStore = signalStore(
    { providedIn: 'root' },
    withState(initialState),

    withComputed((store) => ({
        currentQuery: computed(() => {
            const table = store.selectedTable();
            if (!table) return null;

            const tableInfo = store.tables().find(t => t.name === table);
            if (!tableInfo) return null;

            const includedColumns = tableInfo.columns
                .filter(c => !store.excludedColumns().includes(c.name))
                .map(c => c.name);
            
            if (includedColumns.length === 0) return null;

            return `SELECT ${includedColumns.join(', ')} FROM ${table}`;
        }),

        currentColumns: computed(() => {
            const tableName = store.selectedTable();
            if (!tableName) return [];

            const table = store.tables().find(t => t.name === tableName);
            return table?.columns ?? [];
        }),

        currentRows: computed((): Record<string, unknown>[] => {
            const tableName = store.selectedTable();
            if (!tableName) return [];
            return (store.results()[tableName] ?? []) as Record<string, unknown>[];
        }),

        columnHeaders: computed(() => {
            const tableName = store.selectedTable();
            if (!tableName) return [];

            const rows = store.results()[tableName] ?? [];

            if (rows.length === 0) return [];
            return Object.keys(rows[0] as Record<string, unknown>);
        })
    })),

    withMethods((store, toast = inject(ToastStore)) => ({
        async load() {
            if (store.isLoaded()) return;

            try {
                const tableNames = await invoke<{ name: string }[]>('get_database_tables');

                const tables: DatabaseTable[] = await Promise.all(
                    tableNames.map(async (t) => {
                        const columns = await invoke<DatabaseColumn[]>('get_table_columns', { tableName: t.name });
                        return { name: t.name, columns };
                    })
                );

                toast.show('success', 'Database tables loaded successfully');
                patchState(store, { tables, isLoaded: true });
            } catch (error) {
                toast.show('error', 'Failed to load database tables');
                patchState(store, { isLoaded: false });
                throw error;
            }
        },

        selectTable(tableName: string) {
            patchState(store, { selectedTable: tableName, excludedColumns: [] });
        },

        toggleColumn(columnName: string) {
            const excludedColumns = store.excludedColumns();

            if (excludedColumns.includes(columnName)) {
                patchState(store, { excludedColumns: excludedColumns.filter(c => c !== columnName) });
            } else {
                patchState(store, { excludedColumns: [...excludedColumns, columnName] });
            }
        },

        async executeQuery() {
            const query = store.currentQuery();
            if (!query) return;

            patchState(store, { isQuerying: true });

            try {
                const results = await invoke<Record<string, unknown>[]>('execute_query', { query });
                patchState(store, { results: { [store.selectedTable()!]: results }, isQuerying: false });
            } catch (error) {
                toast.show('error', 'Failed to execute query');
                patchState(store, { isQuerying: false });
                throw error;
            }
        }
    })),

    withHooks({
        onInit: (store) => {
            effect(() => {
                const query = store.currentQuery();
                if (query) {
                    store.executeQuery();
                }
            });
        }
    })
)