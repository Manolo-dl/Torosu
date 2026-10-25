// Mirrors src-tauri/src/commands/system/update.rs.

/** A newer version on the update server. */
export interface UpdateInfo {
    version: string;
    currentVersion: string;
    /** Release notes. */
    notes: string | null;
    date: string | null;
}

export interface UpdateProgress {
    downloaded: number;
    total: number | null;
}
