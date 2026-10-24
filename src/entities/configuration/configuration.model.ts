export interface AppConfig {
    osu_path?: string;
    excluded_tools: string[];
    downloader: DownloaderConfig;
    packs: PacksConfig;
    sqlite_db: boolean;
    open_file: boolean;
}

export interface PacksConfig {
    /** Copy each diff's background into the pack. */
    keep_background: boolean;
    /** Above this many original sets (each with its own audio and background) the pack is split; 0 never splits. */
    max_sets_per_pack: number;
}

export interface DownloaderConfig {
    notify: boolean;
    error_file: boolean;
    /** The saved cookie; set from the downloader, and only kept with `save_session`. */
    osu_session?: string;
    /** Keep the osu_session cookie between sessions; off, it's forgotten when Torosu closes. */
    save_session: boolean;
    output_path?: string;
    load_library: boolean;
    skip_video: boolean;
    max_concurrent_downloads: number;
}